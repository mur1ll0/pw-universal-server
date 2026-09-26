# Estado atual e como retomar

> Nota de passagem entre sessões: onde o trabalho está, o que já é fato verificado e qual é
> a fila. **Atualizar ao fim de cada bloco de trabalho** — e manter curto: o relato
> detalhado de cada sessão (sintoma, causa com referência ao fonte, correção, provas) vai
> para o `docs/HISTORICO_DE_SESSOES.md`, com número de item. **O cabeçalho não é diário:** um
> bloco novo entra como linha no §3.3 (o que olhar em jogo), no §5 (o que ficou faltando) e
> no índice do §8 — não como parágrafo aqui.
>
> **Última atualização: 2026-09-26**, revisão do estado depois do **B120**. Último commit:
> `9ae52ec` (B112–B119); o **B120** (Chamado da Raposa e Muralha de Espinhos) está na árvore,
> **não commitado**.
>
> **Publicado em 2026-09-26 11:10 (−03):** `pw-realm-126`, `pw-world-126`, `pw-realm-155` e
> `pw-world-155` foram reconstruídos a partir da árvore que já tinha o B120 — ou seja, **tudo
> até o B120 está nos dois realms**, nada disso visto em jogo ainda (roteiro no §3.3). Até
> então o último publicado era o 155 de 2026-09-22 (B79–B86).
>
> **Scripts de banco de 2026-09-24 a 26:** todos com efeito no banco (conferido em
> 2026-09-26: Tsuko com atributos base 5 e teto de chi 99, nenhuma linha de dicas desligada
> sobrando). Nenhum pendente.
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

**Marco de 2026-09-26 (B120): o 1.5.5 está jogável no básico; o 1.2.6 carrega os dados
próprios (`elements.data` v7, `tasks.data` v55, `ptemplate.conf`, `clsconfig`, habilidades do
`gs` 1.2.6) e ganhou, junto com o 1.5.5, o mascote de combate.** Tudo está na `main`. Ordem
combinada com o Murillo:

1. ~~1.5.5 jogável no básico~~ — **atingido** (2026-09-23). O que falta dele está em §5A/§5B e
   continua na fila, mas deixou de bloquear.
2. **1.2.6 com o que o 1.5.5 já resolveu** ← **estamos aqui** (painel em §5D). Desde o B101 o
   trabalho é guiado pelos testes do Murillo com a **Tsuko** (Feiticeira/Bárbaro 1.2.6).
3. Depois: banco de dados (Contexto E), painel `pw-admin` (G), atualizador/launcher (H).

**O que o 1.5.5 faz hoje** (realm `realm_155`, porta 29004, cliente BR v156, `elements.data`
v156, `tasks.data` 129, build 2569):

- **Conta e personagem:** login, criação com os atributos, a arma e a habilidade iniciais da
  classe, nascimento no mapa 161 num servidor de mundo próprio, troca de personagem, saída.
- **Mundo:** NPCs, monstros, recursos e outros jogadores entram e saem por distância; altura
  pelo `.hmap`, água pelo `watermap/`, estrutura pelo `movemap/`; os três `.data` do realm
  lidos inteiros, fechando no último byte.
- **Movimento:** andar, voar (item de voo com máscara de classes), meditar e sentar (dobra a
  regeneração), gestos, montaria (sessão de invocar/recolher, não monta na água funda e cai
  dela), teleporte por transportadora com os pontos descobertos (`world_targets.sev`), modo
  roupa que persiste.
- **Combate:** golpe normal com dano adiado como no original, monstros com dados do
  `MONSTER_ESSENCE` que perseguem desviando de obstáculo (porte do `pathfinding`), voltam,
  passeiam, renascem num ponto novo da área e atacam sozinhos quando agressivos; habilidades
  com conjuração, execução, recarga, chi e conjurar andando; formas (Sombria, Raposa); cura e
  PvP; morte e renascimento no distrito; durabilidade das peças.
- **Mascote de combate:** invocar, seguir, atacar (ordem/defesa/automático), habilidades,
  morrer, reviver, experiência, fome e lealdade, soltar, renomear, aprender/esquecer
  habilidade, gravação na jaula (B111–B115).
- **Progressão:** experiência, nível, cultivo pela missão, pontos de atributo, treinador (que
  consome o livro), Daimon (ficha e experiência).
- **Missões:** do `tasks.data` com listas binárias, automáticas por zona, dinâmicas,
  prêmios (itens, dinheiro, experiência, cultivo, teto de chi), monstro invocado, itens de
  missão retirados na entrega.
- **Itens e economia:** loja de NPC com o preço do arquivo, drop e
  coleta (inclusive mina que acorda monstro), poções no tempo com recarga por família, amuleto
  e hierograma automáticos, flechas, descarte com destrave de slot, reparo, caixa de Cartas de General.
- **Social:** fala, grupo.

**O que falta no 1.5.5** (§5A): o sistema de Cartas de General (a caixa já dá a carta),
serviços de refinar e incrustar, os ~300 efeitos de habilidade sem porte, intérprete do
`aipolicy.data`, trava de PvP, resto do Daimon, armazém, troca de mapa entre contêineres,
fôlego debaixo d'água.

**O que o 1.2.6 faz hoje** (realm `realm_126`, porta 29000): as **regras de jogo são as
mesmas** — ficam no `pw-gs`, e o que muda por versão fica no `WorldProtocol` de
`crates/pw-protocol/src/versions/v126/` e nos dados do realm. Loga, cria personagem (moldes do
`clsconfig` 1.2.6: atributos 5/5/5/5, barras, nascimento junto ao Guia, Portal da Cidade),
entra no mundo, combate e experiência conferidos byte a byte com captura, ficha pelo
`ptemplate.conf` 1.2.6, habilidades com tempos, mana, alcance e dano do `gs` 1.2.6, missões
do v55 (filhas, automáticas, lugar a alcançar, itens retirados, teto de chi), venda, bênçãos
visíveis (`ENCHANT_RESULT` de 16 B) e o mascote de combate. **Nada disso depois do B100 foi
visto em jogo** — a publicação de 2026-09-26 é a primeira com esses blocos. Painel em §5D.

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
| `realm_126` | 1.2.6 | 29000 | `pw-world-126` (1) | `data/realm_126/config` | loga, entra, combate, missões e mascote pelo `gs` 1.2.6 (testado, publicado em 2026-09-26); **frente atual** (§5D) |
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

# O realm 1.2.6 do mesmo jeito: pw-world-126 pw-realm-126

# Logs
docker logs -f pw-realm-155        # login, entrada no mundo, o que o link trata
docker logs -f pw-world-155        # os mapas 1 e 161
```

**Referência da suíte, medida em 2026-09-26 (B121, árvore com o B120) com o banco:** **746 testes: 744 passaram, 0 falhas, 2 ignorados** (97 binários; intermitentes sob carga, passam isolados: os de grupo, 1177, hierograma)
(`cargo test --workspace --no-fail-fast -- --test-threads=2`). O limite de dois fios evita
contenção no pool do Postgres nos testes de mundo. Qualquer falha nova deve ser investigada.

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

Os itens abaixo passaram na suíte com o banco. **Todos estão publicados** nos realms 126 e 155
desde 2026-09-26 11:10 (−03). Até o B86 o Murillo declarou o 1.5.5 jogável sem apontar defeito
neles, mas não há relato item a item. Detalhe e roteiro de cada um no histórico.

**1.5.5 (realm 155, e o que vale para os dois)**

| item | o que olhar |
| :--- | :--- |
| B69–B70 | chi: +5 por golpe, +15/s meditando, teto 99 sem aviso repetido |
| B71 | poção sem tela de cultivo; recarga por família (vida, mana, antídoto) |
| B72 | combate longo sem pausa no minuto do autosave |
| B73–B74 | Flecha Glacial −25 chi, Barreira de Asa −45 e escudo de 20 s; amuleto automático |
| B75–B76 | Daimon ganha 1/10 da experiência; Flor de Safira solta o Guardião; monstro agressivo |
| B77–B80 | modo de combate, Atq. Mágico na ficha, montaria (canalizar, montar, desmontar), quem chega vê montado |
| B82–B86 | conjurar andando (2571/2579 do Tormentador), modo roupa persiste e o dono se vê de roupa, descarte de item |
| B87–B88 | montaria recusada na água funda (erro 81) e derrubada acima de 1 m |
| B95 | Forma Sombria (2570) transforma por 19 s (nível 1), tranca o equipamento e desfaz no fim; a Caixa de Tesouro do Guerreiro (41073) vira uma carta |
| B97 | monstros de área no chão nascem e andam **em cima** de estrutura (Gárgulas na pedra do mapa 161, tela 486/525) |
| B99 | monstro de chão contorna obstáculo e estrutura ao perseguir, voltar e passear; cerca o alvo em vez de empilhar |
| B104–B106 | (os dois) monstro passeando sem "correr/pular/disparar"; corpo morto some e o monstro renasce ~15 s depois noutro ponto da área; sair de combate libera o personagem |
| B111–B115 | (os dois) mascote de combate: invocar sem "montar", seguir sem o som do andar recomeçar, atacar, habilidades com recarga, morrer/reviver (329), Curar Mascote (330), soltar, renomear, aprender/esquecer no NPC; na plataforma do Ancião o mascote não afunda |
| B113 | (os dois) aprender habilidade no treinador consome o livro; sem livro, erro 22 |
| B118 | (os dois) sentado, a regeneração dobra a partir do 2º segundo |
| B120 | (os dois) Chamado da Raposa (312): vira raposa, a barra troca, as habilidades fora da forma são recusadas, a 312 de novo desfaz. Muralha de Espinhos (306): o monstro corpo a corpo perde vida a cada golpe que acerta |

**1.2.6 (realm 126, Tsuko e WRA)**

| item | o que olhar |
| :--- | :--- |
| B98, B100 | skill 299: 85 → 88 → 142 → 123, animação e efeito depois da canalização (88 em ~1,5 s, 123 em ~2,5 s) |
| B100 | NPC 3518 oferece a missão inicial 1177 (sem "missão não disponível") |
| B101 | Feiticeira nível 1 com físico 4-4, mágico 6-6; atributos 5/5/5/5; contador de abate sobe; 299 tira ~20–24 no nível 1 |
| B102 | 1177 ativa só a filha escolhida; lista de missões aparece; barras preenchidas e nascimento junto ao Guia da raça em personagem novo |
| B107 | missão automática do jogador novo (9376) entregue; tela de dicas do jogador novo aparece |
| B108 | Portal da Cidade (167) na barra de todas as classes |
| B109 | vender não deixa itens sombreados; Baú de Tesouros da 3427/3428 dá o Sinal de Refinamento |
| B110 | 5911 "Instruções" cumpre ao chegar ao lugar e a 5912 vem; fim do laço da 5909 |
| B116 | Curar Mascote e toda bênção/maldição aparecem (sem "FALHA"); vender vários itens de uma vez não sombreia; jogar fora e usar livro sem descarte no overlay |
| B117 | entregar missão tira os itens de missão da bolsa; invocado da mina não fica na faixa de mascote |
| B119 | teto de chi 99 na Tsuko e chi enchendo no combate; meditar não dá chi no 1.2.6 (é da versão) |
| B120 | Tsuko: a 312 vira raposa (sem tempo), a barra troca, a 299 é recusada, a 312 de novo desfaz; 306 com ícone e o monstro corpo a corpo perde vida a cada golpe |

Onde olhar: `docker logs --since 10m pw-world-126 2>&1 | grep -iE "sem porte|recus|mascote|missão"`
(e `pw-world-155` para o 1.5.5). No overlay (`d_rtdebug 1`), qualquer linha é um comando
descartado — o esperado é silêncio.

---
## 4. Dados de referência

| arquivo | leitor | estado | usado pelo mundo? |
| :--- | :--- | :--- | :--- |
| `elements.data` | `pw-data-loader/src/generic_elements.rs` (+ `specs/elements_layouts/pw_elements_reader.py`) | v7 do `realm_126`: **119 entradas, 23.337 registros, 16.664.770 bytes** (B94); v156 do `realm_155`: **231/231** tabelas; v159 do EN: 234/234 antes de sair (B55). Fecham no último byte, sem override | armas, armaduras, acessórios, monstros (com drop), NPCs e seus serviços de missão e habilidade, classes, poções, preços, pilhas, curva de exp, ajuste por nível, perda na morte (spec 03 §3.1). coleta (`MINE_ESSENCE`, B51), munição. **Não ligados:** `WEAPON_SUB_TYPE`, `NPC_SELL_SERVICE`, `NPC_TRANSMIT_SERVICE` |
| `tasks.data` | `pw-data-loader/src/tasks.rs` | **14.885/14.885** raízes no `realm_155` (B45); **2.819/2.819** no `realm_126`, 7.994 tarefas, projeção parcial (B96); ambas fecham pelos offsets | **sim** — motor de missões (`pw-gs/src/missoes.rs`, B50); v55 ainda requer mapeamento dos demais campos |
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
   37 efeitos portados (B53), mais `Wingshield` (B73), `Fairyform` (B95, Forma Sombria),
   `Foxform` e `Retort`/`Retort2` (B120) e `Rebirth`/`Decregiondmg` (B115) — falta o
   `EventChange` (as passivas próprias da forma); os ~300 outros (invocação,
   escudos, recargas) sem porte; imunidades de monstro; talentos. O **Portal da Cidade** (167) não tem efeito (B17c).
7. **Guia do jogo** do personagem novo: a barra de atalhos agora é gravada pelo próprio
    cliente (B51); o `config_data` do molde no `clsconfig` (barra pré-preenchida) segue não
    decodificado (B44b15, B47d).
8. **Sem trava de PvP:** qualquer jogador machuca qualquer outro, em qualquer lugar
    (`bus_server.rs`, comentário em `pvp`). O original exige duelo, guerra ou mapa de PK
    (B35d).
9. **Mascote de combate:** montaria (B78-B80) e combate (B111/B112: invocar, IA, habilidades,
    soltar, renomear, aprender/esquecer) testados na suíte; falta ver em jogo, e faltam
    invisibilidade, água/ar e bênção de mascote em outro alvo. Do estado estendido, os bits que
    ainda vão zerados são os que não temos dado para preencher — emote, efeitos visíveis,
    facção, barraca, cônjuge, título, VIP e os outros do `state2` (spec 04).
10. **Daimon (B75/B76, parcial):** tem ficha e ganha experiência, e só. Faltam o equipamento e
    as habilidades dele, o vigor, as pílulas de experiência, a decomposição, o refino,
    distribuir pontos de atributo e de gênio, e o bônus sorteado de 10 em 10 níveis. **O ganho
    truncar para zero com o Daimon muito abaixo do dono é do original** (B76), não é defeito.
12. **Cartas de General (B95, só a caixa):** abrir a caixa (`POKER_DICE_ESSENCE`) dá a carta
    com o bloco certo. Falta o sistema: equipar a carta, liderança, os atributos que ela dá,
    subir de nível, devorar (`swallow_exp`) e renascer.
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
- Obstáculo: o monstro **de chão** desvia como o original desde o B99 (`navegacao.rs`, porte
  do `pathfinding`); monstro de **água e ar** ainda anda em linha reta (os agentes
  `ChaseInWaterPF`/`ChaseOnAirPF` e o mapa aéreo `airmap/` não foram portados).
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

Evidência do 1.2.6: o **`gs` 1.2.6** (ELF com símbolos, `files1.2.6/pwserver/gamed/gs`) e o
`libtask.so` 1.2.6 — a fonte mais forte desde o B100; fontes 1.5.3 (o protocolo base);
capturas da VM 1.2.6 em `docs/evidencias/126/` (lidas com
`cargo run -p pw-pcapdiff -- <pcap> --interno --subcomando N`); binário do cliente 1.2.6
(validador de ids: ids > 260 são recusados, VA 0x584618); `docs/INVENTARIO_PROTOCOLO_126.md`
(98 S2C e 46 C2S do `pw-gs`, com o que já foi conferido).

| área | estado no 1.2.6 | o que falta | onde |
| :--- | :--- | :--- | :--- |
| login, criação, entrada | **testado** (B63, B74-126); moldes das 6 classes pelo `clsconfig` 1.2.6 (atributos, barras, nascimento, 167) aplicados no banco (B101, B102, B108) | ver em jogo | `docs/ENTRADA_126.md` |
| combate (layouts 24/26/33/83/84/144) | **testado** byte a byte com captura (B89) | ver em jogo | `docs/COMBATE_126.md` |
| experiência e itens (31/36/46/72/99/156/158, 181, venda de 12 B) | **testado** (B90, B93, B109, B116) | preço real da loja no 1.2.6 (v7) e ver em jogo | `docs/ITENS_EXPERIENCIA_126.md` |
| `OWN_EXT_PROP` (152 B) | **confere byte a byte** com a captura (B93) | — | `specs/04` |
| comandos 14 e 64 | **divergem** da captura | medir e sobrescrever no v126 | inventário |
| `elements.data` v7 | **testado no leitor genérico** (B94, B100): 119 entradas, **23.447 registros**, 16.664.770 bytes; ordem e `sizeof` das 118 tabelas pelo `gs` 1.2.6; agressividade **confirmada em jogo** | conferir no `gs` os prefixos só plausíveis (STONE, PARAM_ADJUST, TASKDICE, SECONDLEVEL) e os campos opacos | `specs/03` §3.1 |
| `tasks.data` v55 | **testado no Rust** (B96): 2.819 raízes, 7.994 tarefas, fecha no último byte; lidos filhas (B102), automática/nível/pré-missões/gênero/zona (B107), lugar a alcançar (B110), `m_bClearAcquired` (B117), teto de chi do prêmio (B119) | os campos fixos ainda não mapeados; ver as missões iniciais em jogo | `specs/03` §3.2 |
| ficha e atributos | **testado** (B101): `ptemplate.conf` com as 8 seções do `gs` 1.2.6; `svr_monster_killed` de 9 B | ver em jogo | `specs/03` §3.5/§3.10 |
| habilidades | **testado**: tempos, mana, aprendizado, alcance e dano do `gs` 1.2.6 (B100, B101); `ENCHANT_RESULT` de 16 B com modificador em 2 bytes (B116, B118); `allow_forms` do construtor do `gs` (B120) | **65 roteiros herdados do 1.5.5 divergem do `gs` 1.2.6** (B115, ver abaixo); ver a 299 em jogo | `specs/05` §habilidades, `specs/habilidades_126/` |
| mascote de combate | **testado** (B111–B116): `PET_ESSENCE` v7 pelo `gs`, invocar/recolher/HP/ataque com os tamanhos do 1.2.6, Curar Mascote só cura (sem `Rebirth` no 1.2.6) | ver em jogo | `specs/05` §8.2 |
| comandos acrescentados depois do B76 no 1.5.5 | parcialmente conferidos pelo caminho (181, 102/232, 233/234, 252, bênçãos) | conferir o resto: `PLAYER_MOUNTING` (227), pet op (235/236), `SET_COOLDOWN` (198), estado estendido do `info_player_1` (inclusive o bit de forma), bit `MODA` do `SELF_INFO_1`, descarte (C2S 14/15) | spec 04 |
| sistemas que o 1.2.6 não tem | Daimon: `ELF_EXP` (283) omitido (B93); meditar não dá chi (B119); sem `Rebirth`/`Decregiondmg` (B115) | conferir cultivo e o que mais não existe no cliente 1.2.6, e omitir pelo trait | validador do cliente |
| dados de mapa do `realm_126` | `.hmap`, `watermap/`, `movemap/`, `world_targets.sev`, `npcgen.data`, `precinct.sev` | conferir que cada leitor fecha no último byte com os arquivos do 1.2.6 | `data/realm_126/config` |
| publicação e teste em jogo | **publicado em 2026-09-26** com tudo até o B120 | o teste do Murillo com a Tsuko (roteiro no §3.3) | skill `pw-testar-e-publicar` |

**Roteiros de habilidade do 1.2.6 (B115):** o catálogo 1.2.6 herda do 1.5.5 os
`StateAttack`/`BlessMe`; 65 divergem do `gs` 1.2.6 em efeito (ex.: 610 reduz ataque/magia lá e
defesa/resistência no herdado; 404/406/… dão `Ap` e não `Apgencont2`; 58/59 não fazem nada
lá). Lista: `python specs/habilidades_126/conferir_roteiros_126.py ../files1.2.6/pwserver/gamed/gs 70`.
Caminho: estender o emulador do extrator aos `StateAttack`/`BlessMe` (sondando `L` e as
variáveis) e gerar os roteiros do 1.2.6; enquanto isso, casos pontuais em `ROTEIROS_DO_GS_126`.

**Forma de raposa (B120):** faltam as passivas `EVENT_CHANGE` (323: +50% de nado; 324:
`SetIncfight(0,2 + 0,1·L)`) aplicadas só na forma (`SkillWrapper::EventChange`,
`skillwrapper.cpp:589-610`) e o bit de forma no `info_player_1` do 1.2.6 (quem entra no campo
de visão depois não vê a raposa; medir o `CheckValid` do cliente 1.2.6). Os espinhos só
devolvem golpe normal de monstro; golpe de habilidade (o `value` do `Retort2`) e de jogador
não passam por eles.

**Diferenças conhecidas ainda abertas:** teto de 80 criaturas visíveis contra 220 no original
(spec 05 §5.0.9, B103); a Batatinha (15955) com o nome dentro do modelo é dado do cliente
(B116), sem correção no servidor.

**Próximos passos, em ordem:** (1) **teste em jogo do Murillo** com o que foi publicado em
2026-09-26 (§3.3) — o que ele relatar passa à frente de tudo; (2) commitar o B120 (a pedido);
(3) roteiros do 1.2.6 gerados do `gs` (os 65 do B115); (4) passivas da raposa e bit de forma
no `info_player_1`; (5) conferir os comandos restantes do B77–B88 no 1.2.6; (6) comandos 14 e
64; (7) completar os campos do v55 e os prefixos do v7.

**Outras frentes, depois:**
- **Cliente v181** (`E:\0_GAMES\Perfect World`, build 2591): exigiria `v181.json` no
  catálogo e um pacote de servidor da mesma build (B11).
- **Servidor 1.5.5 original numa VM 32-bit** (`pwserver_155v156`): o gabarito da versão
  certa (B44c).
- Banco (E), `pw-admin` (G), atualizador/launcher (H) — memória `pw_roadmap_contextos`.

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
| 94 | 09-23 | catálogo `elements.data` v7 no leitor genérico; campos do 1.2.6 e teste do realm |
| 95 | 09-24 | Forma Sombria (`Fairyform`: forma pelo `PLAYER_CHGSHAPE` 163, equipamento trancado com erro 40, velocidade e defesa) e a caixa de Cartas de General (`POKER_DICE_ESSENCE` → carta de 32 B) |
| 96 | 09-23 | Leitor Rust do `tasks.data` v55 (2.819 raízes/7.994 tarefas, corrupção); missão 1173 aceita no teste; agressividade confirmada, poção aparentemente correta em jogo, efeito da skill pendente |
| 97 | 09-24 | leitor do `movemap/` (`.rmap` + `.dhmap`): nascimento de área no chão e passo do monstro em cima da estrutura (Gárgulas do mapa 161) |
| 98 | 09-24 | skill 299 no v126: 85 → 88 → 142 → 123 para o dono; 88 removido do broadcast; visual do próprio Tsuko pendente em jogo |
| 99 | 09-24 | desvio de obstáculo do monstro de chão: porte do `pathfinding` (perseguição dispersa sem bloqueio + `CPf2DBfs`, passeio com o agente de 30 nós), volta para casa com `ReturnHome` |
| 100 | 09-24 | (1.2.6) missão inicial 1177 (`NPC_TASK_OUT_SERVICE` v7 pelo `gs`) e tempos das habilidades do `gs` 1.2.6 |
| 101 | 09-24 | (1.2.6) `ptemplate.conf` do `gs` 1.2.6, atributos 5/5/5/5, `svr_monster_killed` de 9 B, mana/alcance/dano das habilidades |
| 102 | 09-24 | (1.2.6) missões com filhas, moldes do `clsconfig` (barras e nascimento), concluídas no `TASK_DATA`, catálogo por realm |
| 103 | 09-24 | (1.2.6) diagnóstico: monstros "teleportando" não reproduzido, dano da 299 e prêmios corretos |
| 104 | 09-24 | (1.2.6) monstros correndo/pulando no passeio: dois `OBJECT_MOVE` a ~50 ms |
| 105 | 09-24 | (1.2.6) corpo pelo `iDeadTime` e renascimento em ponto novo (a Planta Devoradora) |
| 106 | 09-24 | (1.2.6) passeio sem `OBJECT_MOVE` final e saída de combate com `SELF_INFO_00` |
| 107 | 09-24 | (1.2.6) missão automática do v55 e tela de dicas do jogador novo |
| 108 | 09-24 | (1.2.6) Portal da Cidade (167) nos moldes pelo `clsconfig` |
| 109 | 09-24 | (1.2.6) `UNFREEZE_IVTR_SLOT` na venda e o Baú de Tesouros do "Teste de Salto" |
| 110 | 09-25 | (1.2.6) lugar a alcançar do v55 (5909/5911/5912) |
| 111 | 09-25 | mascote de combate: invocar, IA, morte, experiência, fome, reviver, jaula; `PET_ESSENCE` v7 |
| 112 | 09-25 | mascote de combate: habilidades (comandos 4/5, recarga 252, erro 93), soltar (102 → 232), renomear (NPC 36), aprender/esquecer (38/37); `InfoPet::do_bloco` lia 40 de 192 B e a gravação zerava nome e habilidades. |
| 113 | 09-25 | o treinador consome o livro da habilidade (`GetRequiredItem`), 1.2.6 e 1.5.5 |
| 114 | 09-25 | mascote: som do andar (sem parada a cada passo), `FindGroundPos` ao invocar/reposicionar (plataforma do Ancião), Curar Mascote no mascote |
| 115 | 09-25 | Curar Mascote: `Rebirth`/`Decregiondmg` no 1.5.5; 1.2.6 só cura (roteiro do `gs`); 65 roteiros 1.2.6 divergentes achados |
| 116 | 09-26 | 1.2.6: `ENCHANT_RESULT` 16 B (bênçãos visíveis), item vendido de 12 B (sombreado), `PLAYER_DROP_ITEM` pela versão; Batatinha = dado do cliente |
| 117 | 09-26 | 1.2.6: `m_bClearAcquired` no v55 (itens de missão não saíam) + limpeza SQL da Tsuko; id do invocado fora da faixa de mascote |
| 118 | 09-26 | sentar dobra a regeneração; "FALHA" nas bênçãos 1.2.6 (modificador em 2 bytes); chi, alcance e dano do mascote conferidos |
| 119 | 09-26 | 1.2.6: teto de chi do prêmio v55 (+40), Tsuko com 99; meditar sem chi no 1.2.6; corrige o B118 |
| 120 | 09-26 | Chamado da Raposa (`Foxform` + `allow_forms`) e Muralha de Espinhos (`Retort`/`Retort2`) nas duas versões; regeneração do mascote conferida |
| 121 | 09-26 | revisão do estado e da documentação depois do B120: publicação de 09-26 medida, scripts conferidos no banco, `README.md` reescrito |
