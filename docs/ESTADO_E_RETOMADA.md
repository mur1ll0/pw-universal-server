# Estado atual e como retomar

> Nota de passagem entre sessões: onde o trabalho está, o que já é fato verificado e qual é
> a fila. **Atualizar ao fim de cada bloco de trabalho** — e manter curto: o relato
> detalhado de cada sessão (sintoma, causa com referência ao fonte, correção, provas) vai
> para o `docs/HISTORICO_DE_SESSOES.md`, com número de item. **O cabeçalho não é diário:** um
> bloco novo entra como linha no §3.3 (o que olhar em jogo), no §5 (o que ficou faltando) e
> no índice do §8 — não como parágrafo aqui.
>
> **Última atualização: 2026-10-07**, base B193 commitado (`a642665`) + B194–B195 locais; **B122** — roteiros e passivas do 1.2.6 gerados do `gs`,
> comandos do B77–B88, 14 e 64 conferidos e sobrescritos no 1.2.6, campos do v55 pelo
> `libtask.so` e prefixos do v7 pelo `gs`; **B126** — IA de combate do monstro por estratégia
> (conjurar de longe, série de golpes, afastar, fugir, fixo), eventos de vida 75/50/25 %, efeito
> das habilidades de monstro e o número do dano no tempo (`HURT_RESULT`/`BE_HURT`); **B127** —
> intérprete do `aipolicy.data` (gatilhos por tempo, vida, começo de combate, acaso; fala de
> monstro; controladores do `npcgen` em jogo); **B128** — correções do teste da Tsuko: monstro
> desiste pelo `aggro_time` e volta invencível com a vida cheia, detecção por `sight_range`,
> dano do ar pela metade, velocidade do item de voo, pegar do chão na bolsa comum e compra no
> NPC do 1.2.6; **B129** — roupa com o conteúdo do original (o sexo exigido). Últimos commits:
> `d6a6c4a` (B120) e `ff778c1` (revisão, B121); **B130** — captura de mascote (Domesticar
> Animal); **B131** — mascote de ar segue voando; **B132** — monstro renasce sem o sangramento.
> **B122 a B132 commitados em 2026-09-27** (`2b7b621`); **B133 a B137 commitados em 2026-09-27**
> (`bd25437`): espaço aéreo e `nofly` (B133), monstro que desiste de quem não alcança (B134),
> dano/alcance/altura do mascote (B135), mascote de ar sem afundar nem travar + rotas do
> `path.sev` + grupo e chefe (B136), punição de nível, ódio pelo `GetEnmity`, redução/esquiva de
> dano e roubo de vida (B137). As imagens `pw-world-126` e `pw-world-155` de 2026-09-27 12:04
> têm até o B135; a `pw-world-126` de 2026-09-27 23:37 tem até o B137 (a Tsuko testou o B136/B137
> nela). **B138 a B141 commitados em 2026-09-28** (`3daad2d`): mascote de ar no terreno (B138),
> alcance do corpo a corpo do monstro com o corpo do alvo (B139), atordoado/preso/selado no
> mascote (B140), veneno no mascote sem ódio pelo tique (B141). Não publicados.
> **B157 (2026-09-30)** — missões na morte do jogador (`OnTaskPlayerKilled`): a 990 do cultivo 39
> chega ao morrer depois da 923. **B158** — morrer voando/montado pousa e desmonta. Não publicados.
> **B142 (2026-09-28):** oito ajustes de fidelidade do §5B — reparo pelo `repairfee`, carimbo
> `crc_e` com a tabela do original e `EQUIP_DATA_CHANGED` (67) a quem vê, crítico e esquiva de
> dano no `attack_flag`, `PLAYER_DIED` a quem vê, saldo sem duplicata, `NPC_INFO_LIST` ao C2S
> 68, direção do jogador, mana só nas asas de Arqueiro/Anjo. Commitado em `6b84bcb`.
> **B143 (2026-09-28):** paridade do 1.2.6 — voo e efeitos visíveis no `state` (grupo no
> 1.5.5), `m_ulType` do v55 (sem `m_bItemNotTakeOff` no v55), espinhos contra habilidade e
> jogador, sentado pelo `StayInCommandHandler` e maldição que levanta, preço do vendedor (×1,05 e
> arredondamento), opacos do v7 nomeados, mapas do 126 fechando, vista por fatias sem teto.
>
> **Publicado em 2026-09-26 11:10 (−03):** `pw-realm-126`, `pw-world-126`, `pw-realm-155` e
> `pw-world-155` foram reconstruídos a partir da árvore que já tinha o B120 — ou seja, **tudo
> até o B120 está nos dois realms**, nada disso visto em jogo ainda (roteiro no §3.3). Até
> então o último publicado era o 155 de 2026-09-22 (B79–B86).
>
> **Republicado em 2026-09-26 ~19:32 (−03)** (imagens de 22:32Z): o log do `pw-world-126` mostra
> a Loja Gold (B125) e monstro usando habilidade (B126), então **B122–B127 estão publicados** e
> foram os que a Tsuko testou. **O B128 não está publicado.**
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

**Marco de 2026-09-26 (B122): o 1.5.5 está jogável no básico; o 1.2.6 carrega os dados
próprios (`elements.data` v7, `tasks.data` v55, `ptemplate.conf`, `clsconfig`) e, desde o B122,
tem os roteiros de habilidade e as passivas do próprio `gs` 1.2.6; ganhou, junto com o 1.5.5, o
mascote de combate.** Tudo está na `main`. Ordem
combinada com o Murillo:

1. ~~1.5.5 jogável no básico~~ — **atingido** (2026-09-23). O que falta dele está em §5A/§5B e
   continua na fila, mas deixou de bloquear.
2. **1.2.6 com o que o 1.5.5 já resolveu** ← **estamos aqui** (painel em §5D). Desde o B101 o
   trabalho é guiado pelos testes do Murillo com a **Tsuko** (Feiticeira/Bárbaro 1.2.6).
3. Depois: banco de dados (Contexto E), painel `pw-admin` (G), atualizador/launcher (H).

**B172–B173 (2026-10-05):** login 126/155 recusado após publicar o painel (conferência de
senha × hashes do `public`); corrigido nos dados em B173 (`admin`/`admin`, `testuser`/`testuser`
como MD5(nome+senha)), migrações aplicadas, link/GS republicados — **falta ver em jogo**.
Antes de publicar: `scripts/conferir_public_antes_de_publicar.sql`. Rumo do painel:
`docs/admin/MEMORIA_DA_REFORMA.md` §6.2–6.3.

**Frente adicional autorizada (B165–B182):** reforma do painel (memória central
`docs/admin/MEMORIA_DA_REFORMA.md`). Testadas na suíte: E1–E4 (login GM, consulta viva,
senha, criação, GM, gold só dar, ban, desconectar), E7 (rates B176, mapas B177) e E5 parcial
(dinheiro, EXP/SP B179; pontos livres, nível e cultivo B182; atributos B184 — E5 fechada); mapas carregados e descarregados
pelo painel em execução, com lista de todos os mapas da versão (B183). Nada disso confirmado em jogo
ainda. E6 em andamento: posição (B185), itens ver/dar (B186) e remover (B187); inventário como o do jogo em andamento (B188 ícones e grade, B189 dica parte 1); **B190 tela nova de personagens** (cartões + tela com janelas); **B191 arrastar** online/offline (e a posição no corpo, `CheckEquipPostion`, agora vale também no jogo). **B192–B193 dica parte 2** (texto dos efeitos como o cliente — frases além do 112 conferidas no binário BR —, linha de classe); **B194 editar item** (modal, edição livre nos valores); **B195 habilidades: ver** (janela com ícones e nomes do cliente). Fila aprovada em 2026-10-07 (memória da reforma §6.14): dica parte 2 → B194 editar item → B195–B196 habilidades → B197 mascotes → B198 missões → B199 aparência; depois E8, E9.
B169: workspace 940/0, 2 ignorados; armazenamento após a última correção 14/0;
49 Python/0 e 5 Node/0. Clientes originais, inspeção visual e imagem Linux pendentes.
B170: workspace 942/4; após correções GS 261/0 (2 ignorados) e canal final 15/0.

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
- **Itens e economia:** loja de NPC com o preço do arquivo, Loja Gold com o cash da conta (B125, falta ver em jogo), drop e
  coleta (inclusive mina que acorda monstro), poções no tempo com recarga por família, amuleto
  e hierograma automáticos, flechas, descarte com destrave de slot, reparo, caixa de Cartas de General.
- **Social:** fala, grupo, com experiência repartida pela equipe a 100 m (B164).

**O que falta no 1.5.5** (§5A): o sistema de Cartas de General (a caixa já dá a carta),
os ~300 efeitos de habilidade sem porte, trava de PvP,
resto do Daimon e troca de mapa entre contêineres (armazém, munição, coleta, produção e trava
de PvP feitos em B144–B147; refinar, incrustar, remover pedras e furar no B163; fôlego: o original não desconta). (O intérprete do `aipolicy.data` foi feito no B127.)

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
| `realm_155` | 1.5.5 | **29004** | `pw-world-155` (mapas 1, 161 e 169) | `data/realm_155/config` | **o realm de teste** — cliente BR; até 2026-09-17 se chamava `realm_155BR` (B55) |
| `realm_126` | 1.2.6 | 29000 | `pw-world-126` (1) | `data/realm_126/config` | loga, entra, combate, missões e mascote pelo `gs` 1.2.6 (testado, publicado em 2026-09-26); **frente atual** (§5D) |
| `realm_153` | 1.5.3 | 29001 | `pw-world-153` (1) | — | abandonado |
| `realm_148` | 1.4.8 | 29002 | `pw-world-148` (1) | — | nunca foi alvo |

Mais `pw-postgres` (5432), `pw-dragonfly` (6379), `pw-auth` e `pw-admin-api` (8000). A
porta do barramento `pw-link`↔`pw-gs` (29100) **nunca** é publicada — não tem autenticação,
e `pw-bus/tests/topologia_do_compose.rs` cobra isso.

Um servidor de mundo por realm com todos os mapas dele (`WORLD_TAGS: "1,161,169"` — o 169 é a Caverna Sombria `a69` da missão 32429, desde 2026-10-01), dados
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

Referência B170 (2026-10-05): workspace **942 passaram, 4 falharam, 2 ignorados**.
Após as correções: GS **261/0, 2 ignorados** (biblioteca 94/0, canal 15/0,
subcomandos 152/0, 2 ignorados); canal novamente **15/0** após preservação de pares opacos.
Suíte integral não repetida após as correções. Schema `test`; Redis não utilizado nesta
rodada Rust. Referência anterior B169: workspace 940/0 (2 ignorados), armazenamento
final 14/0, Python 49/0, Node 5/0; Redis descartável removido.

```bash
# A suíte SÓ testa de verdade com esta variável. Sem ela, os testes de integração
# passam sem verificar nada (memória pw_testes_precisam_do_banco).
TEST_DATABASE_URL="postgres://pw_admin:pw_secure_password_2026@127.0.0.1:5432/pw_database" \
  cargo test --workspace

# Publicar no realm de teste (um link e um servidor de mundo com os mapas 1, 161 e 169)
cd docker && docker compose build pw-world-155 pw-realm-155 \
  && docker compose up -d --remove-orphans pw-world-155 pw-realm-155

# O realm 1.2.6 do mesmo jeito: pw-world-126 pw-realm-126

# Logs
docker logs -f pw-realm-155        # login, entrada no mundo, o que o link trata
docker logs -f pw-world-155        # os mapas 1, 161 e 169
```

**Referência da suíte, medida em 2026-09-28 (B141) com o banco:** **834 testes: 832 passaram, 0 falhas, 2 ignorados** (104 binários, `--no-fail-fast`). Intermitente conhecida de grupo: `o_convite_de_grupo_chega_a_quem_foi_convidado` (~1 em 20; a mensagem diz qual comando chegou a quem convidou).
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

Os itens abaixo foram implementados/testados; a publicação é indicada por item. A base
de 2026-09-26 está publicada nos realms 126 e 155 desde 11:10 (−03). Até o B86 o Murillo declarou o 1.5.5 jogável sem apontar defeito
neles, mas não há relato item a item. Detalhe e roteiro de cada um no histórico.

**1.5.5 (realm 155, e o que vale para os dois)**

| item | o que olhar |
| :--- | :--- |
| B170 | **Local, sem publicação.** Sair/seleção/queda do link → fotografia final salva, entidade e rota removidas. Banco falhando → em transição/reentrada bloqueada; recuperando → saída concluída. Troca incerta não libera login; queda durante a recuperação termina a saída no destino. GS final 261/0, 2 ignorados; canal final 15/0. Workspace anterior 942/4, 2 ignorados, falhas corrigidas nas rodadas específicas. Fencing global/todos os produtores/reinício do GS pendentes; nenhuma edição offline habilitada. |
| B171 | **Local.** Coordenação GM opcional: login não depende de `accounts.revisao_gm` (`#[sqlx(default)]`), registro falho não aborta link/GS, 148/153 fora da coordenação. Workspace 946/1 (falha intermitente de teste de corrida, muda a cada rodada, passa isolado), 2 ignorados. Publicar: migrações `scripts/2026_10_05_*.sql` no `public` primeiro; testar login/saída/troca em jogo. |
| B172 | **Incidente.** Login recusado no 126 e no 155 após publicar B165–B171 (conferência de senha × hashes do `public`). Sem mudança de código; decisão com o Murillo; revisão de rumo do painel em `docs/admin/MEMORIA_DA_REFORMA.md` §6.2. |
| B173 | **Corrigido, falta ver em jogo.** Contas padrão em MD5(nome+senha), migrações no `public` (backup antes), sem `process::exit` na perda do fencing, testes do painel em série. Workspace 946/1 (intermitente de ordem em `subcomandos_no_mundo`, 3/3 isolado), 2 ignorados. Link/GS 126/155 republicados. |
| B174 | **Publicado; falta o Murillo ver.** GM pelo painel corrigido (chaves do canal em `docker/.env`; "não enviado" vira falha 503, não "desconhecido"); GM concedido/removido pela API real nos 4 processos. Interface do painel refeita: home de realms (online/rates), seletor de realm, contas paginadas com gaveta de ações. Node 6/0, Python do painel 49/0. |
| B175 | **Publicado; falta ver em jogo.** Painel: largura total, popup de conta ancorado, alerta padrão, acompanhamento automático. E4 completa: gold (cash da conta, `PLAYER_CASH` reenviado), ban/desban (desconecta ao banir), desconectar (logout com `result` 2). Workspace 951/0, Python 50/0, Node 7/0; ponta a ponta pela API real. |
| B176 | **Publicado; falta ver em jogo.** E7 rates: EXP/SP no abate (independentes), drop = sorteios de item, moedas = rodadas de dinheiro, fração = chance; lidas na partida e trocadas na hora pelo painel (tela Rates). Corrigida leitura de `realms` (`NUMERIC`→`float4`). Workspace 956/0, Python 51/0, Node 7/0. |
| B177 | **Publicado; falta ver em jogo.** E7 mapas: desligar salva e desconecta quem está no mapa, recusa entrada e troca; estado em `realms.config.mapas_desligados`, relido na partida. Tela Mapas no painel. Workspace 955/3 (1 expectativa corrigida, 2 intermitentes antigos de `subcomandos_no_mundo`). |
| B178 | **Corrigido.** GS 126/155 estavam mortos após um `up` interrompido (painel ficava em "Consultando o mundo…"); religados. Painel agora diz "sem resposta" com "Tentar de novo"; skill de publicação confere contêineres depois do `up`. |
| B179 | **Publicado; falta ver em jogo.** E5 primeira fatia: dinheiro (online e offline) e EXP/SP (online) pelo painel, pelo caminho da recompensa de missão; ID deduplicado. Workspace 959/0, Python 53/0, Node 7/0. |
| B180 | **Publicado pelo Murillo.** Gold da conta e dinheiro do personagem: só dar, nunca tirar (recusado no GS, banco, API e interface). |
| B181 | **Feito.** `docker/Dockerfile.core.dockerignore`: o build dos servidores deixa de enviar `target/` (63 GB) e `data/` (4,6 GB) ao Docker. |
| B182 | **Local, não publicado.** Painel → Personagens → ficha. Online: "Pontos livres" 10 → os pontos livres da janela de atributos sobem 10 na hora; "Nível" atual+2 → duas animações de subir de nível, nível novo na ficha, +10 pontos, vida/mana cheias, quem está perto vê a animação; "Cultivo" → efeito de avanço de cultivo e título novo (1.5.5 tem 20–22/30–32; 1.2.6 só 0–8). Offline: os três gravam no banco (ver ao entrar). Nível igual ou menor é recusado. Log: `grep "admin: editar personagem"` |
| B183 | **Local, não publicado; caminho crítico (entrada/troca) — portão §6.2.** Painel → Mapas: lista com todos os mapas da versão (126: 43, 155: 79), ligados primeiro, nomes; filtrar "161" ou "Caverna" e por status; marcar linhas, "Selecionar todos" (só os visíveis), "Desmarcar todos". Ligar selecionados (ex.: 105 Caverna do Fogo) → "Carregando…" e depois "Ligado"; entrar com um personagem e ir até lá. Desligar selecionados com alguém dentro → confirmação, o jogador volta ao login, o mapa some dos carregados; relogar com ele é recusado até religar. Reiniciar o `pw-world-155`: os ligados pelo painel sobem juntos. Log: `grep -E "mapa (carregado|ligado|desligado) pelo painel\|descarregado, tique parado"` |
| B184 | **Local, não publicado.** Ficha do personagem → linha **Atributos** já preenchida com os quatro atuais e "Pontos livres depois". Em jogo: mudar (ex.: força −3, vitalidade +3) e Aplicar → a janela de atributos mostra os valores novos na hora, vida máxima acompanha a vitalidade; **Redistribuir** → todos voltam ao mínimo (1.5.5: 5; 1.2.6: força/agilidade 5, vitalidade/energia 3) e os pontos livres sobem; equipamento com requisito acima do novo valor deixa de valer. Offline grava e aparece ao entrar. Soma acima do total é recusada. Log: `grep "admin: editar personagem"` |
| B185 | **Commitado, não publicado; exceção ao portão do B183 dada pelo Murillo.** Ficha → linha **Posição** (mapa entre os ligados, x/y/z da posição atual). Online no mesmo mapa: mudar X/Z e Mover → o personagem aparece no ponto novo na hora (quem está perto vê ele parado lá); Y vazio cai no chão. Outro mapa (ex.: 161) → tela de carregamento e chega lá; relogar mantém o lugar. Offline: grava e entra no ponto novo. Mapa desligado não aparece na lista; ponto fora do terreno é recusado. Log: `grep "painel: personagem movido"` |
| B186 | **Commitado, não publicado.** Ficha → bloco **Itens**: bolsa, equipamento, armazém e bolsa de missão com nomes. "Dar item": digitar parte do nome (ex.: "poção"), clicar no item, quantidade, Dar. Online: o item aparece na bolsa na hora (um `TASK_DELIVER_ITEM` por pilha: 250 de pilha 100 = 3 entregas; observar no cliente se cada entrega mostra aviso); item de missão vai à bolsa de missão; equipamento chega com propriedades sorteadas (como drop). Pedir mais do que cabe → recusado, nada entra. Offline grava e aparece ao entrar. Log: `grep "admin: editar personagem"` |
| B187 | **Commitado, não publicado.** Ficha → Itens: clicar num item abre a barra **Remover** (quantidade, confirmação). Online, na bolsa ou na bolsa de missão: o item some da bolsa na hora e o jogo mostra a mensagem de que o GM removeu o item (com o nome). Equipamento e armazém com o personagem online: recusado ("só fora do jogo"); offline grava. Se o item mudou de lugar desde a consulta, o painel recusa e pede para atualizar. Log: `grep "admin: editar personagem"` |
| B188 | **Commitado, não publicado.** Ficha → Itens: bolsa, equipamento, armazém e bolsa de missão em **grade com os ícones do jogo** (8 por linha; equipamento com o nome de cada slot). Passar o mouse mostra nome, quantidade, ID e slot; clicar abre a barra Remover. Personagem feminino usa o atlas feminino. Item sem ícone no atlas mostra as duas primeiras letras. Antes de publicar o painel: `data/icones/` precisa existir no host (atlas extraído do `surfaces.pck` 1.5.5; está fora do git). |
| B195 | **Local, não publicado (só painel).** Abrir um personagem → janela **Habilidades**: os ícones das habilidades aprendidas, com o nível no canto; passar o mouse mostra nome (como no cliente BR) e nível/máximo. Comparar com a janela de habilidades do cliente (tecla K). |
| B194 | **Local, não publicado; mexe no GS (canal administrativo).** Personagem **em jogo**, cliente aberto: no painel, clicar numa arma da bolsa → modal Editar item → refino +5, uma pedra no furo, um efeito novo, fabricante → Salvar → "Aplicado no jogo"; **no cliente, sem relogar**, passar o mouse na arma: dica com +5, a pedra e o efeito novo. Mesma coisa numa peça **vestida**: a ficha de atributos muda junto. Mudar a quantidade de uma poção → o número na bolsa do cliente muda. Item do armazém com o personagem em jogo → "Desconectar e aplicar". Offline: editar e entrar → a peça já vem editada. Log do GS: `painel: item editado (online|offline)`. |
| B193 | **Commitado, não publicado (só painel).** Como o B192, agora também para efeitos de Nível de Ataque/Defesa, Força da Alma, "Dano de … reduzido em %", Nível de Matança/Guardião, Espírito, Penet. Física/Mágica: o texto deve ser **idêntico** ao da dica no cliente 1.5.5 BR. Efeito de astrolábio (161–174) aparece "Atributo de erro (id)" — também igual ao cliente BR. |
| B192 | **Commitado, não publicado (só painel).** Personagens → abrir um personagem com equipamento → passar o mouse numa arma/armadura com efeitos: os efeitos em azul-claro com o texto do jogo (ex.: "Ataque físico +25", "Intervalo de Ataque 0.20 segundos"), **iguais à dica dentro do cliente**; efeitos de frase ainda não conferida aparecem como "Efeito N (args)". Peça de outra classe: linha da classe em vermelho antes do nível exigido. Comparar uma mesma peça no painel e no cliente 1.5.5 BR. |
| B191 | **Commitado, não publicado; mexe no GS (equipar).** **No jogo, 1.5.5 e 1.2.6:** arrastar uma arma da bolsa para o slot da cabeça → mensagem "não pode equipar" e a arma fica na bolsa; para o slot da arma → veste normal. **No painel, personagem em jogo:** arrastar item entre slots da bolsa → o item muda de lugar **na hora no cliente**; arrastar a arma da bolsa ao slot Arma → o personagem veste e quem está perto vê; arrastar de volta → tira. Soltar uma poção num slot do corpo → "Esta peça não vai nesse slot". Arrastar do armazém → "Personagem em jogo… Desconectar e aplicar?" → confirmar → o cliente volta ao login e o item muda no banco. **Offline:** qualquer troca entre bolsa/armazém/corpo/missão grava e aparece ao entrar. Log do GS: `painel: item arrastado (online|offline)`. |
| B190 | **Commitado, não publicado.** Personagens → **cartões** (nome, conta, classe com o nome do cliente, nível), 12 por página; buscar pelo nome da conta também acha os personagens dela. Clicar num cartão → tela do personagem com janelas (Estatísticas, Equipamento, Roupas, Inventário, Armazém, Bolsa de missão; Habilidades/Mascotes/Missões "em breve"); o botão da barra recolhe a janela; **‹ Voltar** volta à lista. As edições da E5/E6 continuam as mesmas, agora dentro das janelas. |
| B189 | **Commitado, não publicado.** Ficha → Itens: passar o mouse num item abre a **dica do jogo** — nome na cor do jogo (com "(N Slots)" e "+refino"), nível/velocidade/alcance/dano da arma ou defesa/resistências da armadura, durabilidade (vermelha se zero), requisitos, pedras, efeitos (ainda como "Efeito <id> (valores)"), "Feito por", preço e a descrição do item com as cores. Comparar com a dica do mesmo item no jogo. Antes de publicar o painel: `data/textos/` precisa existir no host (`scripts/gerar_textos_de_itens.py`). |
| B169 | **Local, sem publicação.** Contas globais → selecionar conta → GM global → observar persistência salva/sessões pendentes → consultar ID até aplicação em todos os processos. Sessões mantidas; reentrada necessária para Ctrl+G/coroa. Revogação limpa efeitos e nega comandos nos GS 126/155. Processo indisponível mantém pendência; reinício/mesmo ID recupera. Workspace 940/0 (2 ignorados), armazenamento final 14/0, Python 49/0, Node 5/0; visual/jogo/Linux pendentes; roteiro no manual. |
| B168 | **Local, sem publicação.** Contas globais → Criar conta global → usuário ASCII/ senha → esperar nome minúsculo, ID da conta e operação. Recarregar/relogin/outro realm → Consultar resultado mantém ID; repetir nome com outra caixa/nova operação recusa duplicação. Login da conta comum nos clientes 126/155 deve abrir seleção vazia, sem GM; painel recusa a conta. 47 Python + 117 Rust focados + 4 Node passaram; visual/jogo/Linux pendentes. Migrações só em test; roteiro/log/overlay em `docs/WEB_ADMIN_USER_GUIDE.md`. |
| B167 | **Local, sem publicação.** Contas globais → buscar/selecionar conta → trocar senha → esperar salvo/ID; se timeout, consultar ID após recarregar/relogin, podendo usar outro realm. Senha antiga recusada/nova aceita no próximo login 126/155; sessões de jogo abertas mantidas. Hashes incompatíveis precisam de redefinição explícita antes de publicar link. Migração só em test; roteiro/logs em `docs/WEB_ADMIN_USER_GUIDE.md`, sem pacote novo no overlay. 41 Python + 110 Rust focados + 2 Node passaram; visual/em jogo pendentes. |
| B166 | **Frontend E3 observado no painel em 2026-10-05; consultas vivas sem canal configurado e mundos anteriores.** Após futura publicação do painel e mundos: Personagens → buscar no realm → ficha viva online, persistida após logout; Atualizar acompanha posição/vida/mapa; alternar realm não mistura resultados; falha do GS mantém desconhecido. Mapas/contagem vivos somente com canal configurado. Edição indisponível, inclusive entidade residual após queda do link. Roteiro e logs em `docs/WEB_ADMIN_USER_GUIDE.md`; sem pacote novo/overlay. |
| B165 | **Painel publicado em 2026-10-04, base para teste.** Roteiro em `docs/WEB_ADMIN_USER_GUIDE.md`: login GM, recusa de não-GM, alternância 126/155, recursos indisponíveis e logout. Inspeção visual pendente: nenhum navegador conectado à ferramenta. Sem mudança de pacote do cliente/overlay nesta fatia. |
| B164 | (os dois) **não publicado.** Dois clientes em grupo, perto um do outro (< 100 m do monstro): (1) só um bate e mata → **os dois** recebem experiência na barra; um terceiro membro longe (> 100 m) não recebe. (2) Missão de caça **de equipe** (aceita pelo capitão): cada abate do grupo soma no contador dos dois; fora do grupo ela conta normalmente. (3) Missão de equipe com "capitão conclui, todos concluem": o capitão entrega no NPC → no outro cliente a missão aparece concluída/pronta para entregar. (4) NPC com "item de missão" (serviço 8): com a missão ativa, pedir o item → ele cai na bolsa de missão; sem a missão, mensagem de missão indisponível. Log: `recebeu da equipe de <id> o aviso 3 da missão <n>` e `pediu ao NPC os itens da missão <n>` |
| B163 | (os dois) **não publicado.** No NPC Ferreiro/Refinador: (1) **Incrustar** uma pedra numa arma com furo vazio → a pedra some da bolsa, o dinheiro cai o `install_price`, o tooltip da arma mostra a pedra e o atributo; pedra de grau maior que a arma → mensagem de não incrustar (21). (2) **Remover pedras** → furos vazios no tooltip e o custo descontado. (3) **Refinar** com Pedra Celestial (11208) → janela mostra sucesso/falha; +1 no nome em caso de sucesso; nível ≥1 e falha volta a +0; clicar duas vezes em menos de 1 s → "em recarga". (4) **Furar** (só 1.5.5) com pedras 21043 → "furo feito" e um furo a mais; no 1.2.6 não há a opção. Log: `mundo: <id> incrustou/removeu as pedras de/refinou/furou o item <tid>` |
| B159 | **não publicado.** Painel de GM (Ctrl+G), conta com `gm_privileges` > 0: **Invencível** → mensagem fixa do cliente e monstro/jogador não tiram vida; de novo desliga. **Invisível** → some da tela do outro cliente, não consegue golpear; de novo, reaparece para ele. **Ir até jogador** / **Chamar jogador** com o id (inclusive entre os mapas 1, 161 e 169). **Criar monstro** (id, aparência 0, quantidade, vida em s) → nascem a até 6 m. Log: `grep -E "GM [0-9]+"` |
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
| B124 | (os dois) meditando, vida e mana sobem mais rápido (regeneração com vitalidade/5 e energia/10); apanhar sentado levanta; sentado, recolher/invocar mascote não faz nada (sem travar); morrer raposa renasce raposa e continua vendo a raposa |
| B127 | (os dois) Monstro com política no `aipolicy.data` (645 no 1.2.6, 4.442 no 1.5.5): **fala** no chat e no balão ao entrar em combate/morrer ("Prepare-se para retornar à cidade!!"); usa a habilidade da política no tempo dela (ex.: monstro 7088 conjura a 703 poucos segundos depois de começar o combate); abaixo de metade da vida muda de comportamento; chefe liga/desliga grupos de monstros (controlador). Log: `grep -E "o monstro .* (falou|usou)|controlador|aipolicy — sem porte"` |
| B126 | (os dois) Soco de Uma Polegada (2) e todo dano no tempo: o número aparece sobre o monstro a cada 3 s (antes a vida caía sem número). Monstro de estratégia 3 (a maioria: 2.179 no 1.5.5, 1.216 no 1.2.6) com o alvo longe **conjura** de onde está (animação de canto, depois dano com número de habilidade); perto, bate uma série e conjura; monstro mágico (2) se afasta de quem chega perto; monstro fixo não persegue; monstro sem política no `aipolicy.data` usa a habilidade dos 75/50/25 % de vida ou foge. Log: `grep -E "o monstro .* usou"` |
| B141 | (os dois) **não publicado.** Predador Venenoso (1114) no Vespão ou no Lobo: aparece o veneno sobre o mascote e a vida dele cai a cada 3 s por 15 s (barra do mascote) |
| B140 | (os dois) **não publicado.** Guerreiro Golem (habilidade 37, 75 % de atordoar por 3 s) no Vespão: o Vespão para, não bate nem anda, e aparece atordoado para você; depois volta a bater. Atordoar um monstro que está conjurando cancela a magia dele |
| B139 | (os dois) **não publicado.** Monstro de chão (corpo a corpo) que o Vespão ataca de cima vai até ele e **bate** no Vespão; Tsuko voando baixo (até ~1–1,5 m acima do alcance do golpe) apanha. Voando mais alto que o alcance do monstro, ele continua sem alcançar e desiste — é o original |
| B138 | (os dois) **não publicado.** O Vespão perseguindo monstro que foge por encosta (sobe ou desce morro) acompanha sem parar na crista, sempre acima do chão, e o trajeto desenhado não corta o terreno. Monstros de ar também deixam de "empacar" rente ao chão (erro de porte do B133) |
| B137 | (os dois) **publicado no `pw-world-126` de 27/09 23:37.** (1) Batendo num monstro 10 níveis acima, o dano cai para ~70 % (20 acima: 50 %; 30 acima: 25 %); no mesmo nível, igual. (2) Um monstro que só sangra (sem golpe) não vira contra você pelo tique, e fora de combate recupera a vida. Habilidade com ódio próprio puxa o monstro mesmo sem dano (ex.: as de mascote 747–758, `10 × nível do mascote × (3 + L)`). (3) 1.5.5: roupa com "redução de dano" corta o dano recebido; efeitos de roubo de vida e esquiva de dano funcionam |
| B136 | (os dois) **não publicado.** (1) O Vespão atacando monstro de chão fica **acima** do chão (terreno + 0,2 no mínimo), não entra nele. (2) Com o monstro andando, o Vespão continua perseguindo e batendo. (3) Voando, depois de matar, ele sobe de volta até você. (4) Mapa 1 do 1.2.6, perto de (1506, 2302): o **Carniçal Sanguinário** anda pela rota (um passo por segundo, a pé), com os **dois Fantasmas Malignos** atrás dele (correm quando ele passa de 8 m, passeiam em volta quando está perto). Bater no Carniçal faz os Fantasmas virem em você; os Fantasmas mortos só voltam quando o Carniçal morre e renasce. Log ao subir: `World #1: N rota(s) de patrulha no path.sev`. O Carniçal Violento é outro monstro (solto, 88 áreas, sem rota): ele só passeia 10 m em volta de onde nasce |
| B135 | (os dois) **publicado (imagens `pw-world-*` de 12:04), testado pela Tsuko no 126** — o que continuou errado virou o B136. Vespão Pequeno (Atq 466) bate **mais** que o Filhote de Lobo Feroz (332) — antes tirava ~115 contra ~200 porque levava o corte de 0,5 do ar para o chão, que só vale para jogador. Seguindo você no chão, o Vespão fica 1,5 m acima, não enterrado. No combate ele desce até alcançar o monstro (alcance em 3D) |
| B134 | (os dois) **não publicado.** Batendo de cima (voando) num monstro de chão: depois de ~1–2 s embaixo de você sem alcançar, ele **desiste** — esquece o ódio e volta para casa (invencível se estiver a mais de 10 m); o mascote que bate nele volta a ter o alvo parado. Se o mascote continuar batendo, o monstro vai para o mascote |
| B133 | (os dois) **não publicado.** Monstros de ar (vespas) já **aparecem no ar**, antes do primeiro passo. Monstro e mascote de ar contornam o que é sólido no ar (a octree do `airmap/`) em vez de atravessar. Num mapa `nofly` (instâncias; o mapa 1 voa), decolar dá a mensagem "não pode voar" (55). Log: `airmap: mapa 1 com N octrees` ao subir |
| B132 | (os dois) **não publicado.** Monstro que morre sangrando (ou com qualquer dano no tempo) renasce limpo: sem o ícone do sangramento e sem vir atrás de você (Filhote de Doninha, não agressivo, fica parado) |
| B131 | (os dois) **não publicado.** Mascote de ar (ex.: Vespão Pequeno, `inhabit_type` 2): aparece no ar, 1 m acima de você; segue você voando, subindo e descendo com você; parado, fica no ar (não cai ao chão no cliente). Log: nada novo; no overlay, silêncio |
| B130 | (os dois) **não publicado.** Domesticar Animal (328): no monstro com ovo (ex.: Gato de Presas Afiadas), com a vida baixa, aparece **Sucesso** ou **Falha** sobre ele; no sucesso o monstro some e o ovo entra na bolsa. Monstro sem ovo ou de nível acima do seu: **Imune**. Log: `grep -E "capturou|captura de"` |
| B129 | (os dois) **não publicado.** Roupa comprada (Loja Gold ou NPC) chega com o sexo do molde: o maiô feminino da Tsuko mostra "Feminino" e volta a equipar depois de tirado. As duas peças já gravadas sem conteúdo precisam do SQL do histórico B129 (com a Tsuko fora do jogo) |
| B128 | (os dois) **não publicado.** Monstro que te persegue desiste em `aggro_time` s (15 s nos do começo) se não te alcança — fugindo ou voando — e volta correndo para casa; na volta não leva dano nem nota ninguém, e chega com a vida cheia (no 1.5.5 aparece o efeito de invencível, estado 49; no 1.2.6 o `gs` original não mostra efeito). Agressivo só te nota a `sight_range + tamanho` (6–8 m nos do começo), não a 15 m. Golpe do ar em monstro de chão tira metade. Voo com o item de 15 m/s: 18 m/s (base 3 + 15). Pegar arma do chão vai para a bolsa comum. Comprar no NPC do 1.2.6 funciona |
| B158 | (os dois) **não publicado.** Morrer **voando** (asas ou espada) ou **montado**: na tela de morte o personagem já aparece no chão / a pé; ao reviver, **um** clique no voo decola e **um** na montaria invoca (antes eram dois). No log: `morreu voando — pousou` ou `desmontou` antes do aviso de morte |
| B157 | (os dois) **não publicado.** Cultivo 39: com a 923 "Aparição Vazia de Hades" concluída (ela some logo após aceitar — espera de 0 s, conclusão direta, dá o item 3277 na bolsa de missão), **morrer** oferece a 990 "A Divina de Hades Vazia" (`m_bDeathTrig`). No log: `mundo: … aceitou`/`task_notify_new` da 990 logo após a morte; no diário de missões aparece a 990 com a filha 991 "Demônio Imortal Jeffrey". Missão com `m_bFailAsPlayerDie` ativa falha na morte |
| B155 | (os dois) **não publicado.** Morrer longe da cidade: (1) **Cidade mais próxima** → ~2 s depois o personagem aparece no ponto de renascimento do distrito, com 10 % de vida; outro jogador vê ele sumir do lugar. (2) Morrer de novo com o **Pergaminho da Ressurreição** (3043) na bolsa → **Usar pergaminho**: ~5 s depois levanta **no lugar** com a animação de reviver, fica 5 s invencível, some 1 pergaminho e a recarga de 30 min aparece no item. Sem pergaminho: mensagem de item ausente. Até o nível 9 não perde experiência. Log: `grep "renasceu"` |
| B154 | (os dois) **não publicado.** Invocar o Falcão do Paraíso ou o Filhote de Prata (mascotes **ornamentais**): a barra de 3 s corre e o mascote aparece seguindo a Tsuko, **sem** erro. Não tem atributos de combate (é assim no arquivo e no original), não ataca, não obedece ordem, os monstros o ignoram e golpes não o ferem. Log: `grep "invocou o mascote 1234"` |
| B153 | (os dois) **não publicado; exige o `scripts/2026_09_29_vagas_da_jaula.sql` (já aplicado no banco local).** (1) Com a Tsuko: a jaula mostra **4** vagas (as 4 ocupadas). (2) Na **Gerente de Mascotes** → Restaurar/Reanimar Mascote: escolher um mascote que **não** esteja invocado; a barra de 10 s corre, o mascote some da jaula e aparece o **ovo** na bolsa, com o nível/nome dele no tooltip; cobra o `money_restored` do ovo. Log: `grep "restaurou o mascote"`. (3) Incubar com a jaula cheia: erro e o ovo fica. (4) Missão "Jaula de Mascote" (3327→3330) na Gerente de Mascotes: ao entregar cada etapa, a jaula passa a 2/3/4/5 vagas (a Tsuko só muda na 3330, que dá 5) |
| B152 | (os dois) **não publicado.** Restauração de atributos: com pontos em algum atributo, comprar o "Rest. Superior Total" (12764), falar com o Ancião (Cidade do Dragão) → Reverter Atributos, escolher a opção e OK. Esperar: atributos descem (até 5 no 1.5.5; no 1.2.6 força/agilidade 5 e vitalidade/energia **3**, como o `gs` 1.2.6), os pontos voltam livres na janela do personagem, um item a menos. Repetir no piso: mensagem de erro e o item fica. Log: `grep "restaurou atributos"` |
| B151 | (os dois) **não publicado.** (1) Comprar uma arma no NPC: na bolsa ela **não** fica vermelha e o tooltip aparece, com durabilidade cheia; no log, logo depois de `comprou 1 item(ns)`, não aparece mais `subcomando 53 ... ainda não tratado`. (2) Fabricar a ★★★Nighthawk (15964): sai **300/300**, com a linha "Fabricado por <nome do personagem>" no tooltip e as 3 propriedades fixas do modelo (473, 1008, 1321). A linha de classe **não** aparece para item de todas as classes (máscara 0xFF, igual ao drop do servidor 1.2.6 original na captura): se o item vier vermelho, ver qual requisito o tooltip pinta de vermelho (a foice pede força, agilidade e energia 20). Item comprado **antes** desta versão continua sem bloco gravado (vale o do modelo). Banco: `select length(extra_data) from character_items where item_id=15964` > 0 |
| B148 | (os dois) **não publicado.** Comprar no NPC uma peça de outra classe (vermelha) e tentar vestir: não veste, aparece a mensagem de "não pode equipar" e o item continua na bolsa, sem ficar apagado. Uma peça da própria classe veste normalmente. Log: `grep "sem atender o requisito"` |
| B147 | (os dois) **não publicado.** Armazém: falar com o NPC do armazém e abrir; a janela mostra 16 slots e o dinheiro guardado. Arrastar item da bolsa para o armazém (pilha parcial também), guardar/retirar dinheiro, trocar slots, devolver à bolsa; andar ou Esc fecham a janela. Nada fica apagado/congelado. Relogar e reabrir: itens e dinheiro continuam. Log: `grep -E "armazém"` |
| B146 | (os dois) **não publicado.** PvP: sem ligar a chave (botão de PK), nenhuma habilidade fere outro jogador; com os dois ligados, só com Ctrl. Nível 29 ou menos não liga. Desligar logo depois dá a mensagem de espera. Log: `grep -E "PvP|sem PvP"` |
| B144–B145 | (os dois) **não publicado.** Arqueiro: sem flecha certa o arco não ataca ("não pode atacar"); com a Flecha de Novato o dano sobe. Colher: um golpe interrompe; no 1.5.5 clicar duas vezes seguidas na mina dá "em recarga". RT, missão 31797: a Água Cristalizada fica **na borda** da fonte do mapa 161, não dentro. Forja: no NPC de forja, escolher uma receita com os materiais na bolsa → barra de produção, item na bolsa, materiais e taxa descontados, proficiência sobe na janela de habilidades. Log: `grep -E "produzir|consertou"` |
| B143 | (os dois) **não publicado.** Quem chega depois vê a Muralha/raposa/escudo e quem voa já no ar (1.2.6). Sentado: clicar no chão ou atacar não faz nada; Esc levanta; poção funciona; monstro que amaldiçoa levanta. Muralha de Espinhos: habilidade física de monstro também leva dano de volta (no 1.5.5, 0,02·L do golpe). Loja: o preço cobrado é o que a janela do cliente mostra (ex.: armadura 139 = 10.100). Perto dos Guias, **todas** as criaturas até ~90 m (antes cortava em 80) — observar se o cliente engasga. Log: `grep -E "sentado — comando|fila de .* parada"` |
| B142 | (os dois) **não publicado.** Ferreiro: "consertar tudo" cobra `repairfee × desgaste` (não 150) e a peça volta cheia. Crítico: o número sai **grande** (o `MOD_CRITICAL_STRIKE`). Com dois clientes: A morre, B vê a morte (27); A troca de peça, B vê a troca **na hora** (67 — no 1.5.5 BR o 67 é por analogia com o 66: se B não vir a troca, o overlay de B deve mostrar `Invalid EQUIP_DATA_CHANGED size`); A para virado para um lado, B chega depois e o vê virado para o mesmo lado. Arqueiro com asas: decolar tira mana e o voo tira mana por segundo; sem mana, pousa. Outras classes voam sem gastar mana. Log: `grep -E "consertou|pousou sem mana"` |
| B125 | (os dois) Loja Gold: a janela mostra o cash da **conta** (Tsuko/admin: 1.000.000 → "10000.00"), não o dinheiro do personagem; comprar um item de voo o põe na bolsa e o saldo cai o preço; sem saldo, "dinheiro insuficiente"; item VIP recusado (1.5.5) |

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
| B122 | **não publicado.** Tsuko: na raposa, a ficha mostra precisão +150 % (nível 1) e o dano sobe 30 % (324); ao desfazer, volta. Outro jogador que chega **depois** já a vê raposa, montado ou de roupa. Portal da Cidade (167) leva ao ponto de cidade do distrito. Teleporte e volta para a cidade reposicionam o cliente (o 14 de 16 B). A janela do grupo mostra vida/mana dos membros (64). Montar e desmontar aparece para quem está vendo (227). Jogar fora 2 de uma pilha de 5 deixa 3. Bênçãos 610/404/… com o efeito do 1.2.6; chi das `BlessMe` 404/406/…. Mina permanente (ex.: baú 12858) não some ao ser colhida. Missões com prazo, frequência, depósito, ouro pedido, equipe, casamento e GM passam a ser conferidas |
| B124 | **não publicado.** ícones de bênção aparecem no 1.2.6 (a Muralha, a raposa e as demais — o 125/124 era descartado); golpe do monstro na Muralha mostra o dano refletido nele; os itens da linha B124 do 1.5.5 |

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
   por escala, `FORCE_GIVEUP`, limite de tempo checado só na entrega, força, região que falha.
   Facção recusa sempre (não há sistema de facção). Feitos: invocação, falha por morte (B157),
   coleta com missão (B67), abate em equipe, sucesso/falha da equipe e serviço 8 (B164).
   **Grupo sem limite de membros:** o original tem 10 no 1.5.5 e 6 no 1.2.6
   (`TEAM_MEMBER_CAPACITY`; tabelas de bônus de 11 e 7 entradas) — o nosso convite não confere.
2. **Munição:** sem flecha o golpe não é recusado e o bônus de dano da flecha não entra;
   a compra na loja não confere a lista de venda do NPC. **Itens:** refinar, incrustar, remover
   pedras e furar feitos no B163 (spec 05 §8.3); faltam o furo de acessório (serviço 96),
   transferir refino (45) e addons de habilidade/conjunto (B53).
3. **Coleta:** sem recarga de 500 ms, sem interrupção por dano, exp/SP sem ajuste de nível.
4. **Ataque normal sem animação, e o monstro atacado não reage** (B44 #6). A investigar —
   não medido.
5. **Troca de mapa só dentro do processo** (B51): mapa de outro contêiner exigiria o link
   reencaminhar a sessão. **Teleporte por NPC** (`NPC_TRANSMIT_SERVICE`, destinos por
   waypoint) e GM para outro mapa `falta`. O grupo se desfaz na troca.
6. **Habilidades:** dano (1.123), efeitos no alvo (2.304 roteiros) e em si (266) pelos stubs,
   37 efeitos portados (B53), mais `Wingshield` (B73), `Fairyform` (B95, Forma Sombria),
   `Foxform` e `Retort`/`Retort2` (B120), `Rebirth`/`Decregiondmg` (B115), `Ap`/`Returntown` e as
   passivas `EventChange` da forma de classe (B122); os ~300 outros (invocação,
   escudos, recargas) sem porte; imunidades de monstro; talentos. O **Portal da Cidade** (167) não tem efeito no 1.5.5 (B17c; no 1.2.6 tem, B122).
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
11. **IA de monstro:** estratégias, eventos de vida e o intérprete do `aipolicy.data` feitos
    (B126/B127, spec 05 §4.1); no 1.5.5 faltam invocações, caminhos, ações, histórico e missões
    da política, e as estratégias de ódio por facção/invisibilidade.

**Instâncias (masmorras), diagnóstico B160, `falta` inteiro:** no original, os mapas
`[Instance_isNN]` do `gs.conf` (o 169 é um deles; 52 ao todo) são cópias por **chave**. Sem grupo,
a chave é `(id do jogador, contador)`; em grupo, é a do líder, repassada aos membros
(`GetInstanceKey`, `player_imp.h:2577`). Uma cópia vazia dura `idle_time` (20 min) e cada cópia
tem `life_time` (4 h; `instance_manager.h:164-165`). O batimento é de 10 s
(`instance_manager.cpp:630-700`). Ao relogar, o jogador volta à cópia da chave dele, se ela ainda
existir; se não, entra numa nova, na entrada (`instance_userlogin.cpp:110-227`). Quem sai do grupo
ou excede o limite recebe `kickout_instance` com 60 s e vai para a cidade; a vida em zero
reinicia a chave (`aei_filter.cpp:32-110`, `ResetInstance`, `player.cpp:12925`). Hoje o 169 é
um mapa comum e compartilhado.

**Cultivo automático (robô do cliente), análise B161:** é todo do cliente, sem nada no `gs`.
Botão "AutoRobot" no menu Sistema → `Win_AutoPolicy` → Lua `configs\autopolicy\autokillmonster.lua`.
Ele está presente no cliente BR, com `EnableAutoPolicy = 1` no `uiconfig.ini`. Usa só C2S comuns
(alvo, golpe, habilidade, item, pegar, mover, reviver 4/5). Falta o `REVIVAL_AGREE` (87): aceitar
o reviver de outro jogador. O auto HP/MP exige na bolsa um dos itens 36764–36767
(`HaveHealthStones`, `EC_HostPlayer.cpp:10470`).

**Painel de GM (B159):** do mundo, faltam o 206 (criar item, que precisa ler o
`GM_GENERATOR_ESSENCE`), o 207 (gatilho de gerador), a aparência e o nome do monstro criado, e
os bits de privilégio por comando. Os comandos GNET de GM (expulsar, silenciar, anunciar,
contar online, travar) ainda não existem no link.

### 5B. Fidelidade — números e sinais que ainda não são os do original

- **Feitos no B142:** reparo pelo `repairfee`, `crc_e` e `EQUIP_DATA_CHANGED`, crítico e esquiva
  de dano no `attack_flag`, `PLAYER_DIED` a quem vê, saldo sem duplicata, `NPC_INFO_LIST` ao 68,
  direção do jogador, mana das asas. (`weapon_level`/`attack_speed` da arma já estavam certos
  desde o B41h/B61.)
- A conjuração só usa os 1000 ms fixos quando a habilidade não tem `State1` na tabela do
  servidor.
- O `GetIdModify` (cor de moda, pedras de nível 7+, afiador) não entra nos ids do equipamento
  visível; o golpe de mascote vai sem marca de crítico.
- Voo sem teto; a espada voadora não desconta o tempo de voo do item (`cls_flysword_item::OnFlying`);
  `modo_roupa` e `voando` não persistem (B40c).
- A cura usa o ataque mágico no lugar de `GetMagicdamage`; o Tiro Certeiro (234) assume
  carga cheia (B40c).
- Obstáculo: o monstro de chão desvia desde o B99 e o de ar pela octree do `airmap/` desde o
  B133; o de **água** ainda anda em linha reta (`ChaseInWaterPF` não portado).
- `class_templates` tem colunas de atributo que o código ignora (quem manda é o
  `ptemplate.conf`) — decidir quando o painel for editar moldes (B43h).
- Senha de segurança (`CHECK_SECURITY_PASSWD`): qualquer uma passa — não há senha no banco.
- A lista de venda do NPC (`NPC_SELL_SERVICE`) não é lida — no v7 os nomes vêm do v156 e não
  servem — e a compra não confere se o NPC vende o item (B143).

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
- Sem tratamento: `OPEN_BOOTH` (76, barraca pessoal), dividir pilha
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
| experiência e itens (31/36/46/72/99/156/158, 181, venda de 12 B) | **testado** (B90, B93, B109, B116) | preço do vendedor (×1,05 e `AdjustVendorFee`) desde o B143; ver em jogo | `docs/ITENS_EXPERIENCIA_126.md` |
| Loja Gold (`gshop.data` de 1288 B, C2S 106 com `short×3`) | **testado** (B125) | ver em jogo; sem VIP, brinde nem limite no 1.2.6 (não existem no `gs` 1.2.6) | spec 05 §8, `docs/evidencias/LOJA_GOLD_DIAGNOSTICO.md` |
| `OWN_EXT_PROP` (152 B) | **confere byte a byte** com a captura (B93) | — | `specs/04` |
| comandos 14 e 64 | **sobrescritos no v126** (B122): 14 = 16 B (`pos + tag`), 64 = `6 + 25·n`, igual byte a byte à captura | ver em jogo | `specs/04` §4 |
| `elements.data` v7 | **testado no leitor genérico** (B94, B100): 119 entradas, **23.447 registros**, 16.664.770 bytes; ordem e `sizeof` das 118 tabelas pelo `gs` 1.2.6; agressividade **confirmada em jogo** | sem opacos desde o B143 (`CUSTOMIZEDATA`, `PLAYER_ACTION_INFO`, `FACEPILL` nomeados pelos valores); desde o B122 `CHARRACTER_CLASS_CONFIG`, `PARAM_ADJUST`, `SECONDLEVEL` e `STONE` conferidos no `gs`, `TASKDICE` e `MINE_ESSENCE` corrigidos | `specs/03` §3.1 |
| `tasks.data` v55 | **testado no Rust** (B96): 2.819 raízes, 7.994 tarefas, fecha no último byte; lidos filhas (B102), automática/nível/pré-missões/gênero/zona (B107), lugar a alcançar (B110), `m_bClearAcquired` (B117), teto de chi do prêmio (B119) | desde o B122 os campos que o motor usa pelo `libtask.so` (prazo, frequência, depósito, ouro, equipe, período, facção, exclusivas, espera…); `m_ulType` lido desde o B143 (o v55 não tem `m_bItemNotTakeOff`); falta ver as missões em jogo | `specs/03` §3.2 |
| ficha e atributos | **testado** (B101): `ptemplate.conf` com as 8 seções do `gs` 1.2.6; `svr_monster_killed` de 9 B | ver em jogo | `specs/03` §3.5/§3.10 |
| habilidades | **testado**: tempos, mana, aprendizado, alcance e dano do `gs` 1.2.6 (B100, B101); `ENCHANT_RESULT` de 16 B com modificador em 2 bytes (B116, B118); `allow_forms` do construtor do `gs` (B120) | **65 roteiros herdados do 1.5.5 divergem do `gs` 1.2.6** (B115, ver abaixo); ver a 299 em jogo | `specs/05` §habilidades, `specs/habilidades_126/` |
| mascote de combate | **testado** (B111–B116): `PET_ESSENCE` v7 pelo `gs`, invocar/recolher/HP/ataque com os tamanhos do 1.2.6, Curar Mascote só cura (sem `Rebirth` no 1.2.6) | ver em jogo | `specs/05` §8.2 |
| comandos acrescentados depois do B76 no 1.5.5 | **conferidos** (B122) pelo validador do cliente e o `gs` 1.2.6: 163, 181, 198, 232–236, 252 conferem; **227 = 9 B** (sobrescrito); `info_player_1` com forma, cadáver, roupa e montado (`char cor, int id`); `self_info_1` com roupa; C2S 14 = `u8, u16` (sobrescrito) | voo (0x10) e efeitos visíveis (0x40) no B143; os demais bits são de sistemas que não existem | `specs/04` §4 |
| equipe e missões (B164) | **testado**: repartição de experiência (100 m, piso 0x13, `_team_adjust` de `PARAM_ADJUST+0x1c4` com 7 entradas, classes /20 com 9), `CheckKillMonster` com `bTeam`, `AwardNotifyTeamMem` sem `m_bAllSucc` (`libtask.so` 0xd8be) e serviço 8 (`task_matter_provider`, 4 B) conferidos no `gs`/`libtask.so` 1.2.6 | ver em jogo | `specs/05` §2 e §10 |
| refino e pedras (serviços 10, 11, 35) | **testado** (B163): tamanhos, erros, comandos 92/93/251 e tabelas de refino conferidos no `gs` 1.2.6; talismã sem `binding_only`/`require_level_max`; **furar (47) não existe** no `gs` 1.2.6 | ver em jogo | `specs/05` §8.3 |
| sistemas que o 1.2.6 não tem | Daimon: `ELF_EXP` (283) omitido (B93); meditar não dá chi (B119); sem `Rebirth`/`Decregiondmg` (B115) | conferir cultivo e o que mais não existe no cliente 1.2.6, e omitir pelo trait | validador do cliente |
| dados de mapa do `realm_126` | `.hmap`, `watermap/`, `movemap/`, `world_targets.sev`, `npcgen.data`, `precinct.sev` | todos fecham (B143, `tests/mapas_do_126.rs`) | `data/realm_126/config` |
| publicação e teste em jogo | **publicado em 2026-09-26** com tudo até o B120 | o teste do Murillo com a Tsuko (roteiro no §3.3) | skill `pw-testar-e-publicar` |

**Roteiros e passivas do 1.2.6 (B122, testado):** gerados do `gs` 1.2.6 por `roteiros_126.py`
(0 divergências no `conferir_roteiros_126.py`, contra 64). Novos efeitos no motor: `SetAp` (chi) e
`SetReturntown` (Portal da Cidade 167, que no 1.5.5 segue sem efeito porque lá está no
`State2::Calculate`). Passivas `EVENT_CHANGE` aplicadas na forma de classe. Falta: os espinhos só
devolvem golpe normal de monstro — golpe de habilidade (o `value` do `Retort2`) e de jogador não
passam por eles.

**Sentado (B124):** do `StayInCommandHandler` (`playercmd.cpp:873-1015`) só o mascote está
portado; no original, atacar, conjurar e andar sentado também são ignorados, e a maldição
(`GM_MSG_ENCHANT` não amigável) levanta como o golpe.

**Diferenças conhecidas ainda abertas:** a Batatinha (15955) com o nome dentro do modelo é dado do cliente
(B116), sem correção no servidor.

**IA de monstro (B126):** estratégias, eventos de vida e habilidades de monstro valem nas duas
versões (regra no `pw-gs`, dados de cada realm). O que ainda falta para "monstro usa habilidade
em determinado momento" é o **intérprete do `aipolicy.data`**, feito no B127 (spec 05 §4.1): 1.2.6
com 0 operação/condição sem porte; no 1.5.5 faltam invocações, caminhos, ações, histórico e
missões.

**Rotas e grupos (B136):** o `path.sev` é lido e o monstro com `iPathID` patrulha; grupo e chefe
(`iGroupType` 1/2) têm líder e subordinados. Sem porte: a patrulha de ar/água pela octree (vai em
reta), os gatilhos de rota do `aipolicy` (`PathEnd`, trocar de rota) e o tempo de renascer do
grupo pelo `iFirstGen` (usa o do líder).

**Fila pedida pelo Murillo em 2026-09-28, em ordem, com um commit ao fim de cada fase:**

1. ~~Fidelidade (§5B)~~ — **feito no B142** (falta ver em jogo).
2. ~~**1.2.6**~~ — **feito no B143** (falta ver em jogo): (a) os bits do `state` do 1.2.6 que faltam (`0x2`, `0x40`, `0x400`…); (b)
   `m_ulType` e `m_bItemNotTakeOff` do `tasks.data` v55; (c) espinhos (`Retort2`) contra golpe de
   habilidade e de jogador; (d) sentado: ignorar atacar, conjurar e andar, e a maldição levantar
   (`StayInCommandHandler`, `playercmd.cpp:873-1015`); (e) preço real da loja no v7; (f)
   conferir `CUSTOMIZEDATA`, `PLAYER_ACTION_INFO` e `FACEPILL` do v7; (g) cada leitor de mapa do
   `realm_126` fechando no último byte; (h) teto de 80 criaturas visíveis contra 220.
3. **1.5.5 (§5A)** — em andamento. **Feitos (B144–B146):** munição, coleta, recurso no piso
   (Água Cristalizada), produção no NPC nas duas versões (pedido do Murillo de 2026-09-28), trava
   de PvP; fôlego: o original não desconta (nada a portar); armazém (B147); refinar, incrustar,
   remover pedras e furar nas duas versões (B163); experiência de equipe, abate em equipe,
   sucesso/falha da equipe e item de missão pelo NPC (B164). **Faltam, nesta ordem:** o resto dos
   casos de missão recusados (§5A item 1); troca de mapa entre contêineres (o
   teleporte por NPC já existe, B51); o resto do Daimon; Cartas de General; os ~300 efeitos de
   habilidade; as produções 2–5 e a decomposição.

O teste em jogo do Murillo continua passando à frente de tudo quando chegar um relato.
(B157, 2026-09-30: relato da Tsuko no cultivo 39 — a 990 só chega pela morte; `OnTaskPlayerKilled`
portado. Falta ver em jogo.)

**Outras frentes, depois:**
- **Cliente v181** (`E:\0_GAMES\Perfect World`, build 2591): exigiria `v181.json` no
  catálogo e um pacote de servidor da mesma build (B11).
- **Servidor 1.5.5 original numa VM 32-bit** (`pwserver_155v156`): o gabarito da versão
  certa (B44c).
- Banco (E), `pw-admin` (G), atualizador/launcher (H) — memória `pw_roadmap_contextos`.
- **Painel (B165–B182):** ver em jogo os roteiros B175–B179 e B182; depois E5 (inventário,
  habilidades, missões, posição, aparência, mascotes — cada um com o S2C que atualiza a ficha
  online, senão só offline), E6 subsistemas, E8 moldes de classe, E9 fechamento.

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
| 122 | 09-26 | 1.2.6: roteiros e passivas do `gs` (gerador), `SetAp`, Portal da Cidade, passivas `EVENT_CHANGE`; 14/64/227, `info_player_1`/`self_info_1` e C2S 14 do 1.2.6; campos do v55 pelo `libtask.so`; v7 conferido e `TASKDICE`/`MINE_ESSENCE` corrigidos |
| 124 | 09-26 | 1.2.6: `ICON_STATE_NOTIFY`/`UPDATE_EXT_STATE` no formato do cliente 1.2.6 (ícones sumiam); espinho com `MOD_RETORT`; regeneração com vitalidade/energia; sentado ignora mascote e apanhar levanta; forma reenviada ao renascer |
| 127 | 09-26 | intérprete do `aipolicy.data` (`politica.rs`): gatilhos, timers, variáveis, ódio, habilidade/atacar/fugir, fala (`ChatSingleCast` 94 → `ChatMessage` 80), controladores do `npcgen` em jogo; `mascote.rs` devolvido ao `HEAD` (só forma) |
| 126 | 09-26 | IA de combate do monstro: estratégias 0–7 do `MONSTER_ESSENCE`, `GetPrimarySkill`, `session_npc_skill` (canto, efeito, execução), eventos de vida pelo `RandSelect`, afastar/fugir; `HURT_RESULT`/`BE_HURT` no dano no tempo; diagnóstico do golpe que para com habilidade em recarga |
| 141 | 09-28 | veneno/sangramento no mascote: visível (via B140) e sem ódio pelo tique nem pelo dano direto |
| 140 | 09-28 | atordoado/preso/selado no mascote (a IA obedece e o estado vai a quem vê); canto de monstro cancelado pelo atordoamento |
| 139 | 09-28 | monstro: alcance do `ai_melee_task` (0,6/0,8 do alcance puro + corpos) e o corpo do alvo real (o `size` do mascote) |
| 138 | 09-28 | mascote de ar no terreno: `IsPosPassable` com o ambiente na posição (erro do B133) e, só no mascote, o passo reto sobe o chão em vez de bloquear |
| 137 | 09-27 | punição por diferença de nível no dano do jogador; `GetEnmity` extraído (1.2.6 e 1.5.5) e aplicado, sem ódio pelo tique; redução de dano dos adicionais, esquiva de dano e roubo de vida |
| 136 | 09-27 | mascote de ar: `AdjustCurPos` (terreno + 0,2), alcance do `ai_melee_task`, sessão nova por tarefa; `path.sev` e patrulha; grupo/chefe (líder, `ai_follow_master`, ódio repassado, renascer com o líder) |
| 135 | 09-27 | mascote: sem o corte de camada (`IS_HUMANSIDE` é só jogador), alcance em 3D, 1,5 m acima do dono ao seguir |
| 134 | 09-27 | monstro desiste do jogador que não alcança (`NSRC_ERR_PATHFINDING` → `ClearAggro`, `ai_target_task::OnSessionEnd`) |
| 133 | 09-27 | espaço aéreo (`airmap/`: leitor da octree e o `CNPCChaseSpatiallyPFAgent`), bit `GP_STATE_NPC_FLY` na entrada dos monstros de ar, `nofly` pelo `gs.conf` das duas versões |
| 132 | 09-27 | filtros do monstro saem na morte por golpe normal e por habilidade da tabela antiga; o renascimento tira as maldições (`Reborn`) |
| 131 | 09-27 | mascote de ar/água: `FindValidPos` completo, passo reto 3D, `GP_STATE_NPC_FLY` na entrada, troca de modo quando o dono muda de camada |
| 130 | 09-26 | captura de mascote (`SetEntrap` da 328): chance do original, ovo ao conjurador, monstro some e renasce, marca 0x80/0x100/0x200 no `ENCHANT_RESULT` |
| 129 | 09-26 | conteúdo da roupa (`generate_fashion_item`: nível, cor, **sexo**, etiqueta); roupa e item de voo gerados também na compra no NPC |
| 128 | 09-26 | teste da Tsuko: temporizador de ódio (`aggro_time`), `RollBack` com volta invencível (22) e vida cheia fora de combate, detecção por `sight_range + size`, camada no dano (`gnpc_imp::AdjustDamage`), `speed_increase` do item de voo, pegar sempre na bolsa comum, cabeçalho de compra de 8 B no 1.2.6 |
| 125 | 09-26 | Loja Gold nas duas versões: `gshop.data` do cliente lido (1436/1288 B), `MALL_SHOPPING` 12 B/6 B por versão, cash da conta (`gold_balance`) no `PLAYER_CASH`, `MALL_ITEM_BUY_FAILED` com 3 B |
| 142 | 09-28 | fidelidade: reparo pelo `repairfee`, `crc_e` (tabela do `crc.c`) e `EQUIP_DATA_CHANGED`, crítico no `attack_flag`, `PLAYER_DIED`, saldo sem duplicata, `NPC_INFO_LIST` ao 68, direção do jogador, mana das asas |
| 143 | 09-28 | 1.2.6: voo/efeitos visíveis no `state`, `m_ulType` v55, espinhos contra habilidade e jogador, sentado (`StayInCommandHandler`), preço do vendedor (×1,05, `AdjustVendorFee`), opacos do v7, mapas do 126, vista por fatias sem teto |
| 144 | 09-28 | (em andamento) munição ativa pela faixa da arma e bônus da flecha; coleta com recarga (1.5.5), punição de nível e interrupção pelo golpe |
| 145 | 09-28 | produção no NPC (serviço 12) nas duas versões, proficiência das habilidades de produção, layouts v7 de receita/serviço, recurso sobe ao piso do mapa de movimento (Água Cristalizada na fonte) |
| 146 | 09-28 | trava de PvP (chave 82/83, espera, Ctrl, grupo) e o fôlego que o original não desconta |
| 147 | 09-28 | armazém do personagem (serviço 15, C2S 55–61) |
| 148 | 09-29 | requisito de equipamento ao vestir (`VerifyRequirement`) |
| 149 | 09-29 | bolsa dos cristais da Maestria Elemental — sem mudança (comum é o certo) |
| 150 | 09-29 | material da mina sempre na bolsa comum |
| 151 | 09-29 | compra (NPC e Loja Gold) grava o bloco da loja e fabricação o de `ADDON_LIST_PRODUCE` com o nome do fabricante e durabilidade cheia; `GET_ITEM_INFO_LIST` (53); `v7.json` de arma/armadura/acessório corrigido pelo `gs` 1.2.6 |
| 152 | 09-29 | restauração de atributos (serviço 33) nas duas versões, com o piso 3/3/5/5 do `gs` 1.2.6 |
| 153 | 09-29 | jaula de mascotes com as vagas do original (começa em 1, prêmio `m_ulPetInventorySize`, v55 +36) e restaurar mascote em ovo (serviço 29) |
| 154 | 09-29 | mascote ornamental (`PET_CLASS_FOLLOW`, id_type 8783) invocado como no original, só seguindo o dono |
| 155 | 09-29 | renascer na cidade com `NOTIFY_POS` (e troca de mapa) e com pergaminho no lugar (animação e 5 s de proteção), sessões de 39/99 tiques, nível protegido 9 |
| 156 | 09-29 | teste do passeio do 1.2.6 com o passo de pixel do original (diagonal √2 × pixel) — só teste |
| 157 | 09-30 | cultivo 39 travado na Tsuko: a 990 é de gatilho por morte (`m_bDeathTrig`); `OnTaskPlayerKilled` portado (falha ao morrer + `CheckDeathTrig`) |
| 158 | 09-30 | morrer voando/montado: pouso e desmonte antes do `HOST_DIED` (filtros `REMOVE_ON_DEATH` do `Die`) |
| 159 | 10-01 | mapa 169 (`a69`, Caverna Sombria da missão 32429) no `pw-world-155`; painel de GM: invencível, invisível, ir até/chamar jogador, criar monstro (`debug_command_mode`) |
| 160 | 10-01 | diagnóstico das instâncias (masmorras) do original: chave por jogador/grupo, 20 min ocioso, 4 h de vida, relogar volta à cópia — nada implementado |
| 161 | 10-01 | cultivo automático: robô em Lua no cliente, sem código no servidor; custo = conferir os C2S comuns e o `REVIVAL_AGREE` (87) |
| 162 | 10-01 | monstro que "resetou" no 169: não há ódio infinito em masmorra; as quatro causas de desistência do original; `max_move_range` e o piso de 15 m divergem; motivo do reset não é registrado |
| 163 | 10-02 | refinar (35), incrustar (10), remover pedras (11) e furar (47, só 1.5.5) portados nas duas versões; `EMBED_ITEM`/`CLEAR_TESSERA` corrigidos, `REFINE_RESULT` novo; `combined_services` do layout v7 renomeado |
| 164 | 10-02 | experiência repartida pela equipe (`DispatchExp`/`ReceiveGroupExp`/`player_team::DispatchExp`), abate contado para missão de equipe, `AwardNotifyTeamMem` + `OnTaskForceSucc/Fail` e serviço 8 (item de missão pelo NPC), nas duas versões |
| 165 | 10-04 | reforma do painel: E1 auditada; login PW/GM compartilhado, sessão revogável, UI única e consulta de realms/capacidades; 22 testes Python com banco + 24 Rust; E3–E9 pendentes |
| 166 | 10-05 | painel E3: canal HMAC por realm, mapas/contagem reais e ficha viva/persistida; 37 Python + 6 canal + 3 mapas; regressão 152 passaram/2 ignorados; consultas sem escrita, não publicadas |
| 167 | 10-05 | painel E3/E4: senha global idempotente/recuperável, registro/efeito atômicos e senha validada no link; 41 Python + 110 Rust focados + 2 Node; local, migração somente em test |
| 168 | 10-05 | painel E4: criação global recuperável, conta/ID/resultado atômicos e unicidade por nome sem caixa; 47 Python + 117 Rust focados + 4 Node; local, migrações só em test |
| 169 | 10-05 | painel E4: GM global recuperável, revisão/recibos e caches/efeitos coordenados; cliente exige reentrada; salvamento antes de ban/desconexão; local, migração só em test |
| 170 | 10-05 | saída e queda do link com fotografia atômica; carimbo/exclusão locais contra autosave antigo; transferência recupera confirmação incerta; fencing global/reinício/demais produtores pendentes |
| 182 | 10-06 | painel E5: pontos livres (51), nível direto só subindo (37 por nível) e cultivo (160 difundido; por versão) online e offline; workspace 960/0, Python 53/0, Node 7/0 |
| 183 | 10-06 | painel E7: tela Mapas com todos os mapas da versão (gs.conf + nomes do pwadmin), filtro, seleção em lote; ligar carrega e desligar descarrega o mapa no GS em execução; partida soma `mapas_ligados` |
| 184 | 10-06 | painel E5 fechada: modificar atributos distribuídos (total conservado) e redistribuir ao piso da versão, online (`OWN_EXT_PROP`) e offline |
| 185 | 10-06 | painel E6: mover personagem (mapa ligado, chão do terreno) online pelo `transportar` e offline; exceção ao portão do B183 |
| 186 | 10-06 | painel E6: ver inventário com nomes, buscar e dar item (prêmio de missão em lotes de uma pilha, tudo ou nada) online e offline |
| 187 | 10-06 | painel E6: remover item (online bolsa/missão com `PLAYER_DROP_ITEM` motivo GM; equipamento/armazém offline), slot conferido pelo id |
| 188 | 10-06 | painel E6: ícones dos itens (atlas do cliente 1.5.5 em `data/icones`, recorte DXT1→PNG no painel) e recipientes em grade |
| 189 | 10-06 | painel E6: dica de item como a do cliente (textos do `configs.pck` 1.5.5 BR, bloco do equipamento lido pelo GS); efeitos ainda crus |
| 190 | 10-07 | painel E6/Parte 2: personagens em cartões (todas as contas, busca pela conta, paginação, nome da classe do cliente) e tela do personagem com janelas; fila B191–B199 aprovada |
| 191 | 10-07 | painel E6: arrastar itens online (tratadores do cliente) e offline (transação), "desconectar e aplicar"; `CheckEquipPostion` portado (`posicoes.rs`) e ligado ao equipar do jogo |
| 192 | 10-07 | painel E6: dica parte 2 — texto dos efeitos gerado do `AddOneAddOnPropDesc` (127/157 tipos), `item_ext_prop.txt`, linha de classe e ordem da arma |
| 193 | 10-07 | painel E6: frases da dica além do 112 conferidas no ElementClient BR (`switch` do `AddOneAddOnPropDesc` desmontado); 156/157 tipos com texto |
| 194 | 10-07 | painel E6: editar item — modal, `item_editado.rs` (rabo pelo `alterar_rabo`, cabeçalho por remendo, essência intacta), online com `OWN_ITEM_INFO`, offline no banco |
| 195 | 10-07 | painel E6: habilidades — ver (banco, `skillstr.txt`, ícones do stub do cliente), atlas `Skill`/`Pet` (DXT1/DXT3) |
