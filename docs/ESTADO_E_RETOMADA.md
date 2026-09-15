# Estado atual e como retomar

> Nota de passagem entre sessões: onde o trabalho está, o que já é fato verificado e qual é
> a fila. **Atualizar ao fim de cada bloco de trabalho** — e manter curto: o relato
> detalhado de cada sessão (sintoma, causa com referência ao fonte, correção, provas) vai
> para o `docs/HISTORICO_DE_SESSOES.md`, com número de item.
>
> **Última atualização: 2026-09-14**, B50 (branch `feat/aipolicy-reader`). Reescrito nesta data: o documento tinha 6.500 linhas de diário,
> com "próximos passos" de várias épocas empilhados. O texto antigo está inteiro, sem
> alteração, no `HISTORICO_DE_SESSOES.md`.
>
> **Como o sistema é** (arquitetura, formatos, protocolo, regras de jogo) está nas specs —
> `specs/README.md` é o índice. Este documento diz **onde o trabalho está**.
>
> Referências a itens: **A*n*** e **B*n*** são as duas séries numeradas do histórico (ver o
> cabeçalho dele). Um comentário de código que diga "item 14 do `ESTADO_E_RETOMADA.md`"
> aponta para o histórico.

---

## 0. Em uma tela

**Alvo:** o **1.5.5**, servido pelo realm `realm_155BR` ao cliente 1.5.5 BR
(`elements.data` v156, `tasks.data` 129, build 2569). Ordem combinada com o Murillo:

1. **1.5.5 totalmente funcional** ← estamos aqui.
2. Depois, **1.2.6 totalmente funcional** (loga e entra no mundo; parado desde 2026-09-02).
3. Só depois: banco de dados (Contexto E), painel `pw-admin` (G), atualizador/launcher (H).

O 1.5.3 foi abandonado em 2026-09-02 (o cliente disponível nunca logou); todo o trabalho de
protocolo feito contra os fontes 1.5.3 continua válido, porque o 1.5.5 só acrescenta ids e
campos no fim (A-"MUDANÇA DE BASE" no histórico).

**Onde o 1.5.5 está:** loga, cria personagem, entra no mundo sem crash, vê NPCs, monstros,
recursos e outros jogadores com streaming por distância, anda, voa, fala, compra na loja,
aprende habilidade, bate em monstro e em jogador, cura, e os monstros perseguem, voltam e
passeiam no chão. Os três arquivos de dados do realm são lidos **inteiros**, fechando no
último byte. Personagem novo nasce no mapa 161, servido por um servidor de mundo próprio.

**O B50 (publicado, esperando o teste em jogo)** ligou o laço de jogo: experiência do abate
com subida de nível, regeneração, recarga e tempo de conjuração das habilidades, renascer no
ponto de cidade do distrito, **missões do `tasks.data`** (aceitar e entregar no NPC, contar
abate, premiar), drop de itens e moedas com coleta, compra/venda com empilhamento e aprender
habilidade cobrando SP e moedas. O Arqueiro passa a nascer com flechas. Roteiro na seção 3.4.

**O que mais falta:** troca de mundo (o 161 não leva ao mundo 1), colher recurso, efeitos
de estado das habilidades, intérprete do `aipolicy.data`, trava de PvP, distribuir pontos
de atributo e o resto da seção 5A.

---

## 1. Ambiente

### 1.1 Serviços (`docker/docker-compose.yml`)

Nesta máquina (Windows, Docker Desktop) `docker` e `cargo` rodam direto, sem SSH.
Credenciais na memória `pw_universal_infra_access`.

| realm | versão | porta do cliente | servidor de mundo (mapas) | dados | situação |
| :--- | :--- | ---: | :--- | :--- | :--- |
| `realm_155BR` | 1.5.5 | **29004** | `pw-world-155br` (mapas 1 e 161) | `data/realm_155BR/config` | **o realm de teste** |
| `realm_155` | 1.5.5 | 29003 | `pw-world-155` (1) | `data/realm_155/config` | cliente EN (v159); fora dos testes desde 2026-09-05 |
| `realm_126` | 1.2.6 | 29000 | `pw-world-126` (1) | `data/realm_126` | loga e entra no mundo; parado |
| `realm_153` | 1.5.3 | 29001 | `pw-world-153` (1) | — | abandonado |
| `realm_148` | 1.4.8 | 29002 | `pw-world-148` (1) | — | nunca foi alvo |

Mais `pw-postgres` (5432), `pw-dragonfly` (6379), `pw-auth` e `pw-admin-api` (8000). A
porta do barramento `pw-link`↔`pw-gs` (29100) **nunca** é publicada — não tem autenticação,
e `pw-bus/tests/topologia_do_compose.rs` cobra isso.

Um servidor de mundo por realm com todos os mapas dele (`WORLD_TAGS: "1,161"`), dados
carregados uma vez; o roteador entrega cada jogador ao mapa gravado (spec 02 §2.2).

### 1.2 O que há em `data/realm_155BR/config`

Base: o pacote de servidor `F:\PW\1.5.5\home155\gamed\config` (76 pastas de mapa,
`npcgen.data`, `aipolicy.data`, `.sev`, `gs.conf`, `ptemplate.conf`, `global_api.lua` com
`--102`). Por cima, os 11 `.data` do cliente BR (`elements.data` v156, `tasks.data` 129,
`gshop*.data` …). Mapas de altura `.hmap` por mapa. (B13, B42c, B48.)

O `data/realm_155/config` é o mesmo esquema com os `.data` do cliente **EN** (v159).

### 1.3 Clientes

| cliente | onde | build | serve para |
| :--- | :--- | :--- | :--- |
| **1.5.5 BR** | `F:\PW\1.5.5\1.5.5 BR\` | 2569, elements v156 | **os testes** → 29004 |
| 1.5.5 EN | `F:\PW\1.5.5\1.5.5.EN\` (e `F:\PW\1.5.5\bin`) | 2575, elements v159 | → 29003 |
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
| `F:\PW\1.5.5\home155` | outro pacote de servidor 1.5.5 (2023) | base do `realm_155BR`; tem outro `clsconfig` |
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
cd docker && docker compose build pw-world-155br pw-realm-155br \
  && docker compose up -d --remove-orphans pw-world-155br pw-realm-155br

# Logs
docker logs -f pw-realm-155br        # login, entrada no mundo, o que o link trata
docker logs -f pw-world-155br        # os mapas 1 e 161
```

**Referência da suíte, medida em 2026-09-14 (B50) com o banco:** **536 testes passando e
2 falhando** (`cargo test --workspace --no-fail-fast`) — as duas falhas conhecidas do 1.2.6 no
`pw-data-loader/tests/loader_tests.rs` (`test_elements_data_real_file_if_present` e
`test_game_data_manager_directory_load`: o `elements.data` v7 ainda passa pelo leitor
tipado antigo). Qualquer outra falha é nova.

A suíte cria realms `t_*`, contas e personagens de teste no banco local. **Desde o B49 cada
arquivo de teste apaga, ao começar, o que execuções anteriores deixaram** com mais de 15
minutos (`crates/pw-storage/tests/sql/limpar_sobras_de_teste.sql`) — então o banco guarda
no máximo as rodadas recentes. Em 2026-09-14 foram apagados 3.078 realms de teste com suas
contas, personagens e moldes; backup anterior em
`data/_backups/pw_database_2026-09-14_antes_da_limpeza_de_testes.sql`. Contas reais no
banco: `admin` e `testuser`.

**Scripts de dados do realm** (`scripts/`, aplicados à mão no banco): `asas_e_slot_de_municao_155.sql`,
`corrige_skills_e_armas_155.sql`, `2026_09_08_last_login_at.sql`,
`2026_09_09_atributos_iniciais_por_classe.sql`, `2026_09_11_template_de_classe_155br.sql`,
`2026_09_12_nascimento_no_mapa_161_155br.sql`, `2026_09_14_flechas_do_arqueiro_155br.sql`,
`2026_09_14_listas_de_missao.sql` (tabela `character_task_lists`, também no
`02_MIGRACAO…sql` para bancos novos). Conferido em 2026-09-13: os 12 moldes do
`realm_155BR` estão com `spawn_world_id = 161`.

**Personagens de teste no `realm_155BR` hoje:** **POTATO** (id 4515, Bárbaro, nível 1, mundo
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

### 3.3 Teste em jogo do B48 (2026-09-14)

Confirmado pelo Murillo:
- Arqueiro novo nasce no **mapa 161**, no lugar certo.
- Monstros perseguem no chão, sem atravessar o terreno nem andar no ar, e sem a velocidade
  absurda de antes.

Relatado como defeito:
- **Não dá para aceitar missão.** O Guia dos Alados mostra que tem missão e o diálogo abre;
  ao aceitar, nada aparece. O log do mundo registra `aceitou a missão 18918` e
  `aceitou a missão 32201` — o pedido chega, e a resposta não é aceita pelo cliente.
- **O arqueiro nasce sem flechas.**

Não relatado ainda: passeio dos monstros ociosos.

### 3.4 Publicado, falta ver em jogo (B49 + B50)

Publicado em 2026-09-14 (`pw-world-155br` com os mapas 1 e 161, `pw-realm-155br`). Roteiro:

1. **Arqueiro com flechas:** entrar com o **eaa** (ou criar outro Arqueiro): as Flechas de
   Iniciante (1000) no slot de munição; atacar à distância.
2. **Missão do Guia dos Alados:** falar com o Guia (o diálogo abre **uma vez**), aceitar
   "Escolhido do Chi: Elfo Alado" — tem de aparecer na lista de missões. Ir ao NPC que
   recebe e entregar: +25 exp, +10 SP, +8 moedas. Relogar: a lista tem de voltar igual.
3. **Missão de caça:** aceitar uma de matar monstros, matar — o contador sobe na janela de
   missão; completar e entregar.
4. **Experiência e nível:** matar monstros — o número de experiência aparece e **fica**; ao
   completar o nível, a animação de subir e a vida cheia. Relogar: nível e experiência
   continuam.
5. **Regeneração:** perder vida e parar de lutar — ela volta (mais rápido fora de combate).
6. **Drop:** monstros deixam moedas e às vezes itens no chão; pegar (clique ou tecla de
   pegar tudo). Nos primeiros 30 s só quem matou pega.
7. **Morrer e renascer na cidade:** volta ao ponto de cidade do distrito, com 10 % da vida,
   perdendo um pouco de experiência.
8. **Recarga de habilidade:** conjurar duas vezes seguidas — a segunda é recusada até o
   ícone recarregar.
9. **Loja:** comprar um item (aparece no slot, o dinheiro desce) e vender (o valor do
   arquivo, não 50 fixo). **Treinador:** aprender uma habilidade cobra SP e moedas; sem
   SP, recusa.
10. Do B49: botão armadura/roupa alterna e fica; no log do mundo,
    `pediu a marca das missões dinâmicas: 0x52776c0d`.

Onde olhar se algo falhar: `docker logs pw-world-155br | grep -i "missão\|subiu\|aprendeu\|comprou"`
e o overlay `d_rtdebug` (comando recusado aparece lá).

---

## 4. Dados de referência

| arquivo | leitor | estado | usado pelo mundo? |
| :--- | :--- | :--- | :--- |
| `elements.data` | `pw-data-loader/src/generic_elements.rs` (+ `specs/elements_layouts/pw_elements_reader.py`) | **231/231** tabelas no v156 do 155BR, **234/234** no v159; fecha no último byte, sem override (B46) | armas, armaduras, acessórios, monstros (com drop), NPCs e seus serviços de missão e habilidade, classes, poções, preços, pilhas, curva de exp, ajuste por nível, perda na morte (spec 03 §3.1). **Não ligados:** `MINE_ESSENCE`, `WEAPON_SUB_TYPE`, `NPC_SELL_SERVICE` |
| `tasks.data` | `pw-data-loader/src/tasks.rs` | **14.885/14.885** missões de topo (155BR), 14.978 (155), fecha pelos deslocamentos do cabeçalho (B45) | **sim** — motor de missões (`pw-gs/src/missoes.rs`, B50) |
| habilidades do servidor | `specs/habilidades_155/habilidades.json` (`habilidades.rs`) | 3.316 stubs do `cskill` | recarga, conjuração, custo de aprender (B50) |
| `npcgen.data` | `pw-data-loader/src/npcgen.rs` | v11 lido inteiro; tipo de área, `fOffsetTrn`, extensão de recurso, sem tetos inventados (commit `931b39d`); `a01..a99` (B48) | sim; controladores (`id_ctrl`) tratados como ativos, sem modelar gatilho de evento (B17a) |
| `aipolicy.data` | `pw-data-loader/src/aipolicy.rs` | lido, com o fonte 1.7.2 como autoridade (B27f) | **não** — não há intérprete; o `ai.rs` porta só perseguição, volta e passeio |
| `ptemplate.conf` | `ptemplate.rs` (GBK, não UTF-8) | lido | atributos iniciais por classe; as velocidades dele são valores mortos no original |
| `gs.conf` → `specs/mapas/terreno_155.json` | `specs/mapas/gerar_terreno_155.py`, `terreno.rs` | 79 mapas, pela `tag` (B48c2) | sim, só o mapa que aquele servidor serve (88 MB no mundo 1) |
| `region.sev` / `precinct.sev` | `GameDataManager`, `precinct.rs` | carimbos por mundo; distritos do `precinct.sev` lidos inteiros | `INST_DATA_CHECKOUT`; renascer na cidade (B50) |
| `gshop.data` / `gshop1.data` | só o carimbo | — | `edition` do `Challenge` |
| `gamedbd/clsconfig` | `specs/clsconfig_155/ler_clsconfig.py` | posição de nascimento e vida/mana por classe lidas (B47) | via SQL no `class_templates`. **Não decodificados:** `config_data` (barra de atalhos), inventário, equipamento, habilidades |

---

## 5. O que falta

Conferido contra o código em 2026-09-13 — cada linha diz onde está a evidência.

### 5A. Jogabilidade básica do 1.5.5 — a prioridade

O pedido do Murillo no B44: combate básico inteiro, experiência, alma, moedas, animações,
habilidades com conjuração e recarga certas, mapa inicial e missões iniciais.

1. **Distribuir pontos de atributo** (`SET_STATUS_POINT`): os pontos acumulam desde o B50,
   mas não há como gastá-los.
2. **Missões — o que o motor recusa ou ignora** (spec 05 §10): janelas de horário, região de
   entrega, equipe, facção, casamento, PQ, prêmio por escala, teleporte de prêmio, chegar/sair
   de lugar, falha por morte, item de missão pelo NPC (serviço 8). Missão com janela de
   horário é **recusada** com "fora do horário".
3. **Flechas não são gastas** ao atacar (`DoAttack`, `player.cpp:3066`); a compra na loja não
   confere a lista de venda do NPC.
4. **Ataque normal sem animação, e o monstro atacado não reage** (B44 #6). A investigar —
   não medido.
5. **Troca de mundo não existe.** O link escolhe o servidor de mundo na entrada e nada muda
   depois: portal, teleporte de missão (a "Guarda da Terra" leva do 161 ao mundo 1), GM
   para outro mapa (B48d). Pré-requisito para o começo de jogo no 161 ter continuação.
6. **Só 16 habilidades têm conta** de 3.317 (`habilidades.rs`, `TABELA`); as outras conjuram
   sem efeito. Nenhum efeito de estado (veneno, lentidão, bênção com duração). O **Portal da
   Cidade** (167) não tem efeito (B17c).
7. **Colher recurso não existe:** nenhuma sessão de coleta; o `MINE_ESSENCE` já é lido
    (B41h, B46).
8. **Barra de atalhos e guia do jogo** do personagem novo: a fonte provável é o
    `config_data` do molde no `clsconfig`, ainda não decodificado (B44b15, B47d).
9. **Sem trava de PvP:** qualquer jogador machuca qualquer outro, em qualquer lugar
    (`bus_server.rs`, comentário em `pvp`). O original exige duelo, guerra ou mapa de PK
    (B35d).

### 5B. Fidelidade — números e sinais que ainda não são os do original

- Valor fixo no `bus_server.rs`: reparar custa **150**. A conjuração só usa os 1000 ms fixos
  quando a habilidade não tem `State1` na tabela do servidor.
- `crc_e` (carimbo de equipamento) vai zero: o cliente repede o equipamento a cada
  reaparição (B42k).
- `weapon_level` fixo em 1 e `attack_speed` da arma zerada no bloco do item; o original tira
  o segundo do `WEAPON_SUB_TYPE` (B41h).
- Bits do `attack_flag` desconhecidos: o crítico é calculado e não sinalizado (A-"Na ordem").
- `PLAYER_DIED` (27) não é mandado aos outros jogadores.
- `dir` zerado no streaming de NPC e jogador — a grade guarda posição, não direção (B39e).
- Voo sem custo de mana e sem teto; `GP_STATE_FLY` fora do `state` dos pacotes de visão;
  `modo_roupa` e `voando` não persistem (B40c).
- A cura usa o ataque mágico no lugar de `GetMagicdamage`; o Tiro Certeiro (234) assume
  carga cheia (B40c).
- Monstro atravessa obstáculos: sem mapa de movimento (B48b).
- `class_templates` tem colunas de atributo que o código ignora (quem manda é o
  `ptemplate.conf`) — decidir quando o painel for editar moldes (B43h). O `realm_155`
  (EN) ainda tem só **6** moldes, o mesmo estado que tinha o pareamento de classe trocado
  no 155BR (B43b).
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

### 5D. Depois do 1.5.5

- **1.2.6** (prioridade 2): skills, missões e HP de NPC falhavam no último teste (A62–A63,
  2026-09-02) — muito disso foi resolvido depois, no código comum, mas não retestado lá.
  Nove zonas com `npcgen.data` v5/v6 com conteúdo não são lidas; as duas falhas do
  `loader_tests` são do `elements.data` v7, ainda no leitor tipado antigo (B2).
- **Cliente v181** (`E:\0_GAMES\Perfect World`, build 2591): exigiria `v181.json` no
  catálogo e um pacote de servidor da mesma build (B11).
- **Servidor 1.5.5 original numa VM 32-bit** (`pwserver_155v156`): o gabarito da versão
  certa, que fecharia a classe de problema "o fonte diz uma coisa, o binário faz outra"
  (B44c).
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
   sincronia (A22). Diferença entre versões vai no `PorVersao`.
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
| `CLAUDE.md`, `.claude/agents/pw-server-dev.md`, `.claude/skills/pw-*` | o agente de toda sessão, as skills de trabalho e o hook que cobra a atualização das specs |
| `docs/COMO_TESTAR.md`, `MULTIPLOS_REALMS.md`, `MEDIDAS_DO_126.md`, `CAPTURA_DO_126.md`, `PLANO_ARQUITETURA_E_EXECUCAO.md` | referência |

---

## 8. Índice do histórico (série B, frente 1.5.5)

Para achar rápido o item citado num comentário de código ou numa seção acima.

| item | data | assunto |
| ---: | :--- | :--- |
| 1–3 | 09-02 | `GenericElementsData` no `GameDataManager`; `npcgen.data` v11; realms 155 no compose |
| 4–5 | 09-02/03 | "versão baixa" (`edition` v159 × v156) e "manutenção" (`GAME_VERSION` `0x00010505` lido do binário) |
| 6–7 | 09-03 | `models.pck` de 2 GB, 133 arquivos de mapa faltando, `gshop_ts2`, `region`/`precinct`, `GetUIConfig_Re` |
| 8 | 09-03 | criação do `realm_155BR` |
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
