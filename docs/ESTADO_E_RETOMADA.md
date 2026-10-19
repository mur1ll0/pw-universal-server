# Estado atual e como retomar

> Nota de passagem entre sessões: onde o trabalho está, o que já é fato verificado e qual é
> a fila. **Atualizar ao fim de cada bloco de trabalho** — e manter curto: o relato
> detalhado de cada sessão (sintoma, causa com referência ao fonte, correção, provas) vai
> para o `docs/HISTORICO_DE_SESSOES.md`, com número de item. **O cabeçalho não é diário:** um
> bloco novo entra como linha no §3.3 (o que olhar em jogo), no §5 (o que ficou faltando) e
> no índice do §8 — não como parágrafo aqui.
>
> **Última atualização: 2026-10-15** — **B233: P1 dos recursos do cliente (`pw-recursos`, porta 8100), versão 0 do 1.2.6 e do 1.5.5 importada e conferida por HTTP; commitado em `080a5cb`/`21dfc55`/`bd18444`, **confirmado em jogo nas duas versões**; B234 launcher (P2) em `a43042d`/`a478035`, confirmado; B235/B236 commitados (`07a30ab`…`5a2a22d`); B237 commitado (`d334cf8`…`eaf329e`); B238 commitado (`a78abf2`…`db76f83`); B239 commitado (`c9c54d1`…`cfbe3b2`); B240 commitado (`32e115a`…`7bb1766`); B241 commitado (`92aa98e`, `5c05e53`); B242 commitado (`132c63b`, `66179c2`); B243 commitado (`0353f5c`, `ed573fc`); B244 commitado (`57dc115`, `0194146`); B245 commitado (`2028129`, `3b9c2fb`); B246 commitado (`c338690`, `35ec407`); B247 commitado (`14abd7f`, `f54ef50`); B248 commitado (`3a8bd56`, `39bf94a`); B249 commitado (`f68f47a`, `4f1d5c6`) — **o GS novo (`pw-world-*`) ainda não publicado, falta o Murillo autorizar**; B250 commitado (`0da45ba`, `9783ac8`); B251 commitado (`d3b002b`, `b98b1c5`); B252 commitado (`1c47646`, `ad52efd`); B253 commitado (`e5d8319`, `104847a`) — nenhum mapa alinhado (decisão do Murillo); B254 commitado (`fbc82a9`, `778efa0`); B255 commitado (`ef7a7ba`, `c779028`); B256 commitado (`b578536`, `1c4202e`); B257 commitado (`b1dfe76`, `e9d5ed4`); B261 commitado (`ad60670`, `447a5d8`); **B262: textos longos das missões (descrição, aceitar, recusar, tributo) no Estúdio — publicado em `pw-recursos`/`pw-admin-api`, não commitado.** Antes: B199–B216 commitados (B215 em `4df4376`; B216 em `2263a23`/`804f19e`; B217 Templates em `267d550`/`344788b`; **B220–B228 (habilidades, efeitos, planta, amuleto de dano, auras; produção no 1.2.6) commitados em `9767e82` e no commit de documentação seguinte, não publicados**; **B218, B219 e B229–B232 commitados em 2026-10-14** (E9 da reforma do painel; planos de recursos do cliente, atualizador e Estúdio; 1.2.6 e 1.5.5 rodando sem `.pck`; realms 1.4.8/1.5.3 removidos), não publicados; B212–B214 em `a95e323`/`e5fd20b`); B199–B211 em blocos por área (`50c5d3a` dados, `d0ee485` protocolo, `2d57f7e` GS, `4630bb4` painel, `cecfaac` Docker e o de documentação); **B122** — roteiros e passivas do 1.2.6 gerados do `gs`,
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

**B222 (2026-10-12, local):** a planta invocada (Místico, Feiticeira de seita, classes novas)
inteira — nascer, automática/passiva, grupo, vida, explosão, avisos 293–295 —, a habilidade de
mascote em área, quatro efeitos de planta, a força da alma no roteiro e o empurrão de monstro de
água/ar. `falta`: o `Auracurse` (1794).

**B221 (2026-10-12, local):** os efeitos do Bárbaro no 1.2.6 (tigre, empurrão, provocação,
dano, precisão, veneno, chi) e os de seita da Feiticeira 1.5.5 portados; a planta invocada
(`Summonplantpet`) fica para um bloco próprio.

**B220 (2026-10-12, local):** habilidade instantânea (C2S 80) já não para o personagem (só o
`OBJECT_CAST_INSTANT_SKILL` 178, como o `InstantSkill` original); Traje de Flores (`Swiftform`) e
mais 7 filtros da Feiticeira portados nas duas versões; estados visíveis traduzidos para a numeração
do `gs` 1.2.6; os materiais da produção agora somem da tela no 1.2.6 (o `PLAYER_DROP_ITEM` ia no
layout do 1.5.5). A proficiência de produção acima do nível da receita **não sobe — é o original**.

**B172–B173 (2026-10-05):** login 126/155 recusado após publicar o painel (conferência de
senha × hashes do `public`); corrigido nos dados em B173 (`admin`/`admin`, `testuser`/`testuser`
como MD5(nome+senha)), migrações aplicadas, link/GS republicados — **falta ver em jogo**.
Antes de publicar: `scripts/conferir_public_antes_de_publicar.sql`. Rumo do painel:
`docs/admin/MEMORIA_DA_REFORMA.md` §6.2–6.3.

**Frente adicional autorizada (B165–B182):** reforma do painel (memória central
`docs/admin/MEMORIA_DA_REFORMA.md`). Testadas na suíte: E1–E4 (login GM, consulta viva,
senha, criação, GM, gold só dar, ban, desconectar), E7 (rates B176, mapas B177) e E5 parcial
(dinheiro, EXP/SP B179; pontos livres, nível e cultivo B182; atributos B184 — E5 fechada); mapas carregados e descarregados
pelo painel em execução, com lista de todos os mapas da versão (B183). **E7 confirmada em jogo (2026-10-10, WB
no 1.2.6: rates e mapas).** O resto disso não foi confirmado em jogo
ainda. E6 fechada no B215 (Daimon e edições em jogo; falta ver em jogo); **B218 E9 — a reforma do painel fechou no código** (extensões em `docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md`, manual reescrito, interface revista, tema claro, Daimon escondido no 1.2.6, semeio sem a Poção 1801; falta ver em jogo E4–E6/E8 e publicar); **B217 Templates (E8)**: o painel lê e edita por realm o template de criação de cada classe (estatísticas, bolsa, corpo, habilidades) e o personagem criado depois nasce com ele; janelas recolhidas vão a uma bandeja no canto; **B216 reforma visual da ficha** (janelas iguais, modais, toast, criar item no slot, dar mascote, catálogos; corrige voo 15732 e anel 310 do Tsuko); antes: posição (B185), itens ver/dar (B186) e remover (B187); inventário como o do jogo em andamento (B188 ícones e grade, B189 dica parte 1); **B190 tela nova de personagens** (cartões + tela com janelas); **B191 arrastar** online/offline (e a posição no corpo, `CheckEquipPostion`, agora vale também no jogo). **B192–B193 dica parte 2** (texto dos efeitos como o cliente — frases além do 112 conferidas no binário BR —, linha de classe); **B194 editar item** (modal, edição livre nos valores); **B195 habilidades: ver** (janela com ícones e nomes do cliente); **B196 habilidades: editar** (ensinar, subir, descer, remover); **B197 mascotes** (ver, editar, libertar); **B198 missões** (ver, dar livre, concluir forçando o sucesso, cancelar apagando, esquecer a conclusão). Fila aprovada em 2026-10-07 (memória da reforma §6.14): dica parte 2 → B194 editar item → B195–B196 habilidades → B197 mascotes → B198 missões; aparência fica no cliente (decisão de 2026-10-08, sem editor no painel); depois E8, E9.
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
| `realm_148`, `realm_153` | 1.4.8, 1.5.3 | — | — | — | **removidos no B231** (contêineres, serviços do compose e linhas de `realms`/`class_templates`; sem personagens). `data/realm_148` e `data/realm_153` ficaram no disco, sem uso |

Mais `pw-postgres` (5432), `pw-dragonfly` (6379), `pw-auth`, `pw-admin-api` (8000) e, desde o B233, `pw-recursos` (8100: versão 0 dos clientes e HTTP do atualizador, `specs/07_RECURSOS_DO_CLIENTE.md`; chave no volume `pw_recursos_chave`). A
porta do barramento `pw-link`↔`pw-gs` (29100) **nunca** é publicada — não tem autenticação,
e `pw-bus/tests/topologia_do_compose.rs` cobra isso.

Um servidor de mundo por realm com todos os mapas dele (`WORLD_TAGS: "1,161,169"` — o 169 é a Caverna Sombria `a69` da missão 32429, desde 2026-10-01), dados
carregados uma vez; o roteador entrega cada jogador ao mapa gravado (spec 02 §2.2).

### 1.2 O que há em `data/realm_155/config`

Base: o pacote de servidor `F:\PW\1.5.5\home155\gamed\config` (76 pastas de mapa,
`npcgen.data`, `aipolicy.data`, `.sev`, `gs.conf`, `ptemplate.conf`, `global_api.lua` com
`--102`); desde o B211 os `npcgen.data` de `world`, `a46`, `a61`, `a63`, `a76`, `a77` e o
`a50/precinct.sev` vêm do `pwserver_155v156` (a build v156: o `world` do `home155` não tinha a cidade
da Névoa Sombria); antigos guardados fora do repositório. Por cima, os 11 `.data` do cliente BR (`elements.data` v156, `tasks.data` 129,
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
| `F:\PW\1.7.2\172Source` | fonte do servidor 1.7.2 | nomes de campos que os `.data` do 1.5.5 têm e o fonte 1.5.5 não (`aipolicy.data`, sistema de Lar do `tasks.data`) (B27f, B45) **— não está mais em disco (conferido em 2026-10-14, B237; em `F:` só existe o `1.5.5`)** |
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

**Efeitos de equipamento (B209, 2026-10-08):** com a Tsuko no 1.5.5, os atributos dos efeitos postos pelo
painel passaram a contar ("parecem funcionar") — valor do id, família e realm na edição, `OWN_EXT_PROP` com graus.

**Painel E7 — rates e mapas (B176, B177, B183; 2026-10-10):** com o WB no 1.2.6, alterar as rates reflete
no jogo; ligar pela tela Mapas o mapa de uma masmorra que estava desligada e entrar pelo portal funciona. Na
primeira tentativa o mapa ligado não era o destino do portal (o log mostra `portal recusado … destino 102,
destino_indisponivel` com o 108 ligado): erro de escolha, não do painel.

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

- **B233 (P1 `pw-recursos`, 2026-10-14) — confirmado em jogo pelo Murillo nas duas pastas:** versão 0 importada dos `.pck` originais e servida na 8100; extraída por
  HTTP com conferência de assinatura e hashes em `F:\Games\perfectworld_126 recursos\element` (61.234 arquivos) e
  `F:\PW\1.5.5\1.5.5 BR recursos\element` (327.388 arquivos). **A olhar:** abrir o cliente dessas pastas com o `elementclient.bat`
  (já traz `sepfile`), logar nos realms 126/155 e comparar com as cópias `sepfile` do B229–B232 (roteiro §7.5).
- **B234 (P2 launcher, 2026-10-14) — confirmado pelo Murillo (baixar, Jogar nas duas versões, Verificar):** `tools\pw-launcher\target\release\pw-launcher.exe` (Tauri). As pastas
  `F:\Games\perfectworld_126 launcher\element` e `F:\PW\1.5.5\1.5.5 BR launcher\element` foram instaladas pelo núcleo do
  launcher (com `.pw-atualizacao/`); as `… recursos` do B233 foram apagadas. **A olhar:** abrir o launcher, apontar
  para cada pasta (a situação deve dizer "instalado, versão 0"), Verificar e reparar (0 reparados), Jogar; e instalar o
  1.2.6 do zero numa pasta vazia, fechando o launcher no meio do download e abrindo de novo (deve retomar).
- **B235 (launcher e P3, 2026-10-14):** launcher novo (mesmo `.exe`) com abas de versão, pasta por versão escolhida
  pela janela do Windows, "Baixar o jogo" / "Já tenho instalado"; página **Recursos do cliente** no painel (grupo
  Geral): rascunho → enviar arquivo → publicar → o launcher mostra "Atualização disponível". **A olhar:** roteiro na
  resposta do B235 (histórico).
- **B237 (Estúdio S1, 2026-10-14):** painel → realm → **Estúdio**: escolher/criar rascunho, tabela, registro, editar,
  salvar, clonar, remover; publicar em Recursos do cliente grava o `elements.data` no `config` do realm. **A olhar:**
  editar o preço de uma arma no 1.2.6, publicar, **reiniciar `pw-world-126`/`pw-realm-126`**, conferir o preço na loja
  do NPC em jogo e, pelo launcher, que o cliente atualizou (mesmo preço no tooltip).
- **B238 (índice de referências):** na ficha do Estúdio, drops e outras referências mostram o nome e abrem o registro
  citado; "Usado por" lista quem cita; listas (drops, páginas de loja, materiais) aparecem em tabela; "Buscar" ao lado
  de uma referência escolhe outro registro do espaço. **A olhar:** um monstro do 1.2.6 → drops com nome; trocar um
  drop pelo "Buscar", salvar no rascunho; tentar remover um tipo de arma (deve recusar dizendo quem o cita).
- **B239 (rótulos e ícones):** a ficha vem em grupos com nomes em português e o ícone do item no alto, nas listas e
  nas referências. **A olhar:** uma arma e um monstro do 1.5.5 — grupos abertos, rótulos, ícones; listas de drop com só
  as linhas cheias.
- **B240 (conversas):** no Estúdio, a tabela `TALK_PROC` lista as conversas; abrir uma mostra as janelas com o texto
  e as opções (para outra janela ou função: loja, teleporte, encerrar…). **A olhar:** abrir um NPC que conversa →
  serviço de conversa → link da conversa; mudar o texto de uma janela, salvar, publicar, reiniciar o realm e conferir o
  texto novo ao falar com o NPC.
- **B242 (mapas):** Estúdio → **Mapas**: escolher o mundo; o minimapa aparece, com zoom (roda) e arrasto; pontos
  vermelhos (monstros), amarelos (NPCs), verdes (recursos); clicar mostra nome, posição e renascer, e "abrir a ficha".
  **A olhar:** se as peças encaixam e se os pontos caem onde as criaturas estão no jogo.
- **B243 (editar o `npcgen`):** Estúdio → escolher/criar um rascunho no topo → **Mapas** → marcar **Editar áreas**: caixas
  das áreas; clicar numa abre os campos e os geradores. Mudar a quantidade ou o modelo, ou "Posicionar pelo clique" e
  clicar no mapa, **Salvar no rascunho**; em Recursos do cliente, publicar o rascunho (item
  `servidor\<mapa>\npcgen.data`), **reiniciar o realm** e conferir no jogo. Reverter a alteração devolve o original.
- **B244 (distritos e portais):** Mapas → marcar **Distritos** e **Portais**: polígonos tracejados azuis com um
  quadrado no ponto de renascer; caixas roxas com linha até a chegada. Clicar abre o editor; com rascunho, mover o
  ponto de renascer ("Posicionar pelo clique"), salvar, publicar, reiniciar o realm, morrer dentro do distrito e
  "renascer na cidade": deve aparecer no ponto novo. Mesmo para a chegada de um portal.
- **B245 (teleportes):** Mapas → **Teleportes**: losangos laranja com o nome do destino; clicar mostra quem leva até
  ele (transportadora, preço, nível, NPCs com link). Com rascunho, mover a chegada, publicar, reiniciar o realm e
  usar o NPC transportador: deve chegar no ponto novo.
- **B246 (rastreador):** Mapas → **Rastreador**: pontos verdes (perto da área do NPC) e vermelhos com linha (a mais de
  20 m). No editor de área de NPC ("Editar áreas"), cada gerador mostra a posição do rastreador e "Usar a posição da
  área no rastreador". Publicar o rascunho gera versão do cliente: atualizar pelo launcher, abrir uma missão que cita
  o NPC e clicar no nome dele — o personagem deve ir ao ponto novo.
- **B247 (controladores):** Mapas → Editar áreas: a caixa "Controladores do npcgen" lista os controladores (busca por
  nome, id ou gatilho); clicar abre o controlador com os horários. Marcar "Objetos" mostra os objetos dinâmicos. Numa
  área com controlador, o editor mostra o controlador com link. Desligar "Ligado ao subir" de um controlador, publicar,
  reiniciar o realm: as criaturas das áreas dele não devem nascer.
- **B248 (altura):** em qualquer editor do mapa com Y (área, renascer, portal, teleporte), "Posicionar pelo clique" e
  clicar: X, Z e **Y** preenchidos; o Y deve bater com o chão no jogo (onde o realm tem `.hmap`; sem terreno, aviso).
- **B249 (reiniciar):** depois de publicar o GS novo: publicar um rascunho com arquivo do servidor → o topo do Estúdio
  mostra "A versão N … reinicie" → "Reiniciar o realm", digitar o id → os jogadores voltam ao login (salvos), o realm
  volta em segundos e o aviso some. No log do GS: `admin: reiniciar — todos salvos e desconectados`.
- **B250 (objetos):** Mapas → Editar áreas → marcar "Objetos": clicar num quadrado azul mostra o nome do modelo (ex.:
  "小风车动画", o moinho), com o caminho do `.ecm` na dica.
- **B251 (distritos/portais novos, 1.5.5):** Mapas → Distritos → clicar num distrito → "Redesenhar polígono" (cliques,
  Concluir) ou "Novo distrito (copiar este)" (cliques, Concluir, nome) → Salvar → publicar (2 itens de servidor e 2 de
  cliente) → reiniciar o realm e **atualizar pelo launcher** → entrar no polígono novo: o nome aparece no cliente;
  morrer lá e renascer na cidade leva ao ponto do distrito. Portal: "Novo portal", "Posicionar a caixa pelo clique",
  destino → publicar, reiniciar, atualizar → andar na caixa leva ao destino.
- **B252 (destinos novos):** Mapas → Teleportes → clicar num destino → "Novo destino (copiar este)" (posicione antes) →
  na ficha da transportadora (NPC_TRANSMIT_SERVICE) pôr o id novo num `targets_N_idTarget` e subir `num_targets` →
  publicar, reiniciar, atualizar → o NPC lista o destino com o nome novo e leva ao ponto.
- **B253 (alinhar):** decidir se o servidor segue o cliente nos 14 pares que diferem (ver §5D e spec 07). Para um mapa:
  Mapas → abrir o mapa → um distrito ou portal → "Alinhar o servidor com o cliente" → publicar → reiniciar. A a61 do
  1.5.5 (onde se nasce) é a primeira candidata: o cliente BR tem 27 distritos, o realm 25.
- **B254 (Loja Gold):** Estúdio → **Loja Gold** → escolher categoria/buscar → abrir uma oferta → mudar o preço da
  compra 1 → Salvar → publicar → reiniciar o realm e **atualizar pelo launcher** → na Loja Gold do jogo a oferta tem o
  preço novo e a compra cobra esse valor.
- **B255 (missões):** Estúdio → **Missões** → buscar "Primeiro Teste" ou 1173 → a ficha mostra o NPC 3517 com nome
  (link para a ficha), filhas, prêmio; navegar pela árvore com os links mãe/filhas.
- **B256 (editar missão):** com um rascunho, na ficha da missão, "Editar" → mudar o prêmio de experiência ou o nível
  mínimo → Salvar → publicar → reiniciar o realm e **atualizar pelo launcher** → no jogo a missão mostra e entrega o
  valor novo.

**1.5.5 (realm 155, e o que vale para os dois)**

| item | o que olhar |
| :--- | :--- |
| B170 | **Local, sem publicação.** Sair/seleção/queda do link → fotografia final salva, entidade e rota removidas. Banco falhando → em transição/reentrada bloqueada; recuperando → saída concluída. Troca incerta não libera login; queda durante a recuperação termina a saída no destino. GS final 261/0, 2 ignorados; canal final 15/0. Workspace anterior 942/4, 2 ignorados, falhas corrigidas nas rodadas específicas. Fencing global/todos os produtores/reinício do GS pendentes; nenhuma edição offline habilitada. |
| B171 | **Local.** Coordenação GM opcional: login não depende de `accounts.revisao_gm` (`#[sqlx(default)]`), registro falho não aborta link/GS, 148/153 fora da coordenação. Workspace 946/1 (falha intermitente de teste de corrida, muda a cada rodada, passa isolado), 2 ignorados. Publicar: migrações `scripts/2026_10_05_*.sql` no `public` primeiro; testar login/saída/troca em jogo. |
| B172 | **Incidente.** Login recusado no 126 e no 155 após publicar B165–B171 (conferência de senha × hashes do `public`). Sem mudança de código; decisão com o Murillo; revisão de rumo do painel em `docs/admin/MEMORIA_DA_REFORMA.md` §6.2. |
| B173 | **Corrigido, falta ver em jogo.** Contas padrão em MD5(nome+senha), migrações no `public` (backup antes), sem `process::exit` na perda do fencing, testes do painel em série. Workspace 946/1 (intermitente de ordem em `subcomandos_no_mundo`, 3/3 isolado), 2 ignorados. Link/GS 126/155 republicados. |
| B174 | **Publicado; falta o Murillo ver.** GM pelo painel corrigido (chaves do canal em `docker/.env`; "não enviado" vira falha 503, não "desconhecido"); GM concedido/removido pela API real nos 4 processos. Interface do painel refeita: home de realms (online/rates), seletor de realm, contas paginadas com gaveta de ações. Node 6/0, Python do painel 49/0. |
| B175 | **Publicado; falta ver em jogo.** Painel: largura total, popup de conta ancorado, alerta padrão, acompanhamento automático. E4 completa: gold (cash da conta, `PLAYER_CASH` reenviado), ban/desban (desconecta ao banir), desconectar (logout com `result` 2). Workspace 951/0, Python 50/0, Node 7/0; ponta a ponta pela API real. |
| B176 | **Confirmado em jogo (2026-10-10, WB no 1.2.6: alterar as rates reflete no jogo).** E7 rates: EXP/SP no abate (independentes), drop = sorteios de item, moedas = rodadas de dinheiro, fração = chance; lidas na partida e trocadas na hora pelo painel (tela Rates). Corrigida leitura de `realms` (`NUMERIC`→`float4`). Workspace 956/0, Python 51/0, Node 7/0. |
| B177 | **Confirmado em jogo (2026-10-10, WB no 1.2.6: ligar o mapa da masmorra e entrar).** E7 mapas: desligar salva e desconecta quem está no mapa, recusa entrada e troca; estado em `realms.config.mapas_desligados`, relido na partida. Tela Mapas no painel. Workspace 955/3 (1 expectativa corrigida, 2 intermitentes antigos de `subcomandos_no_mundo`). |
| B178 | **Corrigido.** GS 126/155 estavam mortos após um `up` interrompido (painel ficava em "Consultando o mundo…"); religados. Painel agora diz "sem resposta" com "Tentar de novo"; skill de publicação confere contêineres depois do `up`. |
| B179 | **Publicado; falta ver em jogo.** E5 primeira fatia: dinheiro (online e offline) e EXP/SP (online) pelo painel, pelo caminho da recompensa de missão; ID deduplicado. Workspace 959/0, Python 53/0, Node 7/0. |
| B180 | **Publicado pelo Murillo.** Gold da conta e dinheiro do personagem: só dar, nunca tirar (recusado no GS, banco, API e interface). |
| B181 | **Feito.** `docker/Dockerfile.core.dockerignore`: o build dos servidores deixa de enviar `target/` (63 GB) e `data/` (4,6 GB) ao Docker. |
| B182 | **Local, não publicado.** Painel → Personagens → ficha. Online: "Pontos livres" 10 → os pontos livres da janela de atributos sobem 10 na hora; "Nível" atual+2 → duas animações de subir de nível, nível novo na ficha, +10 pontos, vida/mana cheias, quem está perto vê a animação; "Cultivo" → efeito de avanço de cultivo e título novo (1.5.5 tem 20–22/30–32; 1.2.6 só 0–8). Offline: os três gravam no banco (ver ao entrar). Nível igual ou menor é recusado. Log: `grep "admin: editar personagem"` |
| B183 | **Confirmado em jogo (2026-10-10, WB no 1.2.6: mapa ligado pela tela Mapas, entrada pelo portal).** Painel → Mapas: lista com todos os mapas da versão (126: 43, 155: 79), ligados primeiro, nomes; filtrar "161" ou "Caverna" e por status; marcar linhas, "Selecionar todos" (só os visíveis), "Desmarcar todos". Ligar selecionados (ex.: 105 Caverna do Fogo) → "Carregando…" e depois "Ligado"; entrar com um personagem e ir até lá. Desligar selecionados com alguém dentro → confirmação, o jogador volta ao login, o mapa some dos carregados; relogar com ele é recusado até religar. Reiniciar o `pw-world-155`: os ligados pelo painel sobem juntos. Log: `grep -E "mapa (carregado|ligado|desligado) pelo painel\|descarregado, tique parado"` |
| B184 | **Local, não publicado.** Ficha do personagem → linha **Atributos** já preenchida com os quatro atuais e "Pontos livres depois". Em jogo: mudar (ex.: força −3, vitalidade +3) e Aplicar → a janela de atributos mostra os valores novos na hora, vida máxima acompanha a vitalidade; **Redistribuir** → todos voltam ao mínimo (1.5.5: 5; 1.2.6: força/agilidade 5, vitalidade/energia 3) e os pontos livres sobem; equipamento com requisito acima do novo valor deixa de valer. Offline grava e aparece ao entrar. Soma acima do total é recusada. Log: `grep "admin: editar personagem"` |
| B185 | **Commitado, não publicado; exceção ao portão do B183 dada pelo Murillo.** Ficha → linha **Posição** (mapa entre os ligados, x/y/z da posição atual). Online no mesmo mapa: mudar X/Z e Mover → o personagem aparece no ponto novo na hora (quem está perto vê ele parado lá); Y vazio cai no chão. Outro mapa (ex.: 161) → tela de carregamento e chega lá; relogar mantém o lugar. Offline: grava e entra no ponto novo. Mapa desligado não aparece na lista; ponto fora do terreno é recusado. Log: `grep "painel: personagem movido"` |
| B186 | **Commitado, não publicado.** Ficha → bloco **Itens**: bolsa, equipamento, armazém e bolsa de missão com nomes. "Dar item": digitar parte do nome (ex.: "poção"), clicar no item, quantidade, Dar. Online: o item aparece na bolsa na hora (um `TASK_DELIVER_ITEM` por pilha: 250 de pilha 100 = 3 entregas; observar no cliente se cada entrega mostra aviso); item de missão vai à bolsa de missão; equipamento chega com propriedades sorteadas (como drop). Pedir mais do que cabe → recusado, nada entra. Offline grava e aparece ao entrar. Log: `grep "admin: editar personagem"` |
| B187 | **Commitado, não publicado.** Ficha → Itens: clicar num item abre a barra **Remover** (quantidade, confirmação). Online, na bolsa ou na bolsa de missão: o item some da bolsa na hora e o jogo mostra a mensagem de que o GM removeu o item (com o nome). Equipamento e armazém com o personagem online: recusado ("só fora do jogo"); offline grava. Se o item mudou de lugar desde a consulta, o painel recusa e pede para atualizar. Log: `grep "admin: editar personagem"` |
| B188 | **Commitado, não publicado.** Ficha → Itens: bolsa, equipamento, armazém e bolsa de missão em **grade com os ícones do jogo** (8 por linha; equipamento com o nome de cada slot). Passar o mouse mostra nome, quantidade, ID e slot; clicar abre a barra Remover. Personagem feminino usa o atlas feminino. Item sem ícone no atlas mostra as duas primeiras letras. Antes de publicar o painel: `data/icones/` precisa existir no host (atlas extraído do `surfaces.pck` 1.5.5; está fora do git). |
| B197 | **Local, não publicado; mexe no GS (canal).** Personagem com mascotes na jaula, **em jogo**, janela de mascotes do cliente aberta: no painel, janela Mascotes → cartões com ícone, nome, modelo, nível; clicar num mascote **recolhido** → nível/lealdade/nome → Salvar → o cliente mostra os valores novos na hora; Libertar → some da jaula do cliente. Mascote **invocado** → "recolha-o antes de editar". Offline: editar e entrar → valores novos. Log: `painel: mascote editado (online|offline)`. |
| — | **Confirmado em jogo pelo Murillo (2026-10-08):** 1.5.5 (Tsuko) — dar, mover, equipar e desequipar itens pelo painel online e offline; editar item (durabilidade, refino, requisitos); dar dinheiro; subir/descer nível de habilidade; ensinar habilidade (Aura de Aço nv 5). 1.2.6 (WB) — dar EXP/SP, subir nível, cultivo, posição, pontos livres, dar missão. |
| B226 | **Local, não commitado nem publicado.** Tsuko (1.2.6) com o Amuleto Bom de Evocação (1233, mágico +30, arma 3–15; a Bandeira-Talismã dela é nível 6): cada habilidade de dano mágico gasta 1 amuleto — a pilha no slot do amuleto diminui — e o número sai maior; o golpe normal não gasta (o amuleto é mágico). Overlay: nada de `Invalid … size` para o 46. |
| B227 | **Local.** Retalhador (Borrão Pontiagudo 1358), Mago (Prisão de Gelo 1806), Guerreiro (Tornado de Lâminas 1816) ou Místico (Bênção de Gaia 1854, Olho Místico 1862): a cada 3 s os monstros perto apanham (ou o grupo recebe a bênção), com o efeito visual de encantamento. |
| B224 | **Local, não commitado nem publicado (rebuild de `pw-world-126` e `pw-world-155`).** Tsuko: costurar de novo a Calça de Couro de Dragão — sai com durabilidade 120/120, a classe em branco, defesa/resistências e os addons de fabricação, e o nome "Tsuko" como fabricante. A calça já feita está certa no banco: relogando ela aparece normal. |
| B222 | **Local, não commitado nem publicado (rebuild de `pw-world-155`; o 1.2.6 não tem planta).** Com um Místico 1.5.5 (ou a Feiticeira de seita): (1) Ferroada da Punição (1389) num monstro — a planta aparece junto dele, entra na barra de mascote e o ataca sozinha; (2) Dissolução (1398) — ela explode e some; (3) Erva de Cura (1393) — a planta passiva cura você/o grupo; (4) invocar duas do mesmo grupo — a primeira some; (5) esperar o tempo — ela some sozinha. Overlay: nada de `Invalid … size` para 293, 294 e 295. Feiticeira: a Destruidor da Alma (1825) põe o Aprisionador de Alma, que deixa lento quem está perto dele e passa parte do dano dele para você. |
| B221 | **Local, não commitado nem publicado (rebuild de `pw-world-126` e `pw-world-155`).** Bárbaro 1.2.6: (1) Rugido do Tigre (112) — vira tigre, ícone, mais rápido, menos dano; lançar de novo volta. (2) Terra Dividida (107) num monstro — ele é jogado ~6 m para trás. (3) Mordida Violenta (150) num monstro que bate em outro — ele vem para você. (4) Força Divina (83) e Sangue Fervente (111): ícones e números da ficha. (5) Presas Venenosas (155): dano de madeira a mais no golpe. (6) Fúria Bestial (188): chi a cada golpe recebido. Feiticeira 1.5.5 de seita: Encantar sela o monstro; Esmagamento Maléfico o paralisa. Overlay: nada de `Invalid … size` para 163 e 35. |
| B220 | **Local, não commitado nem publicado (rebuild de `pw-world-126` e `pw-world-155`).** Tsuko no 1.2.6: (1) andar e apertar a **Traje de Flores** (762) — continua andando, flores no corpo, ícone de velocidade, mais rápida por 5 min; o outro cliente vê a animação e as flores. (2) Costurar um item — os materiais somem da bolsa na hora. (3) Feiticeira: Barreira de Espinhos (310, ícone e menos dano recebido), Ferir/Enfraquecer/Devastar (320–322: uma substitui a outra no monstro), Alma da Raposa (318), Harmonia (311: troca vida↔mana). No 1.5.5 o mesmo da 762. Overlay: nada de `Invalid … size` para 178, 124 e 46. |
| B218 | **Local, não commitado nem publicado (rebuild de `pw-admin-api`, `pw-world-126`/`pw-world-155` para o Daimon e `pw-realm-*` para o semeio).** 1) Painel: menu lateral, topo e popup de contas com ícones de traço (sem emoji); botão de lua no topo alterna claro/escuro e lembra a escolha. 2) Painel do realm 1.2.6: "Mapas no ar" mostra "1 · Mundo"; "Recursos deste realm" sem "Em breve" (edição e templates "Implementado", catálogo "Parcial", editores de dados "Indisponível"). 3) Templates: cartões com "1 · Mundo · 4 itens…". 4) Ficha de um personagem do 1.2.6 (Tsuko/WB): **sem** a janela Daimon; um do 1.5.5 continua com ela. 5) Ficha offline: aviso "a edição é gravada pelo GS, que recusa o login enquanto grava". 6) Realm novo sem template (148/153): personagem nasce com Portal 2100, poções 1796 e 1804 — sem a 1801. Nada no jogo muda além do 4/6. |
| B217 | **Commitado (`267d550`), não publicado; falta ver no painel e em jogo (rebuild de `pw-realm-126`, `pw-realm-155` — o link cria —, `pw-world-126`, `pw-world-155` e `pw-admin-api`; migração `scripts/2026_10_11_templates_e8.sql` já aplicada no `public` e no `test`).** 1) Painel → realm 1.2.6 → **Templates**: seis cartões (Guerreiro … Sacerdote) com nível, atributos, itens e habilidades. 2) Abra o Guerreiro: janelas Estatísticas iniciais, Equipamento, Roupas, Inventário, Habilidades (sem armazém, missão, mascotes, Daimon). 3) Editar estatísticas: nível 5, dinheiro 1000, força 7, Salvar → toast "Template salvo"; vida/mana mudam no cartão da janela. 4) Slot vazio da bolsa → Criar item → uma poção ×5; clique numa peça do corpo → "Tirar do template"; Dar habilidade (lista já filtrada pela classe). 5) Recolha duas janelas: vão para o canto inferior direito e as outras ocupam o espaço; o "+" devolve cada uma ao lugar (vale também na ficha do personagem). 6) Em jogo (1.2.6): crie um Guerreiro → nasce nível 5, 1000 de dinheiro, força 7, com a poção e sem a peça tirada; um Mago não muda. 7) 1.5.5: crie um personagem de qualquer classe sem editar nada → nasce igual a antes (habilidade inicial e Portal da Cidade agora vêm do template). Log do link: `Personagem '…' criado`. |
| B216 | **Commitado (`2263a23`/`804f19e`), falta ver no painel (rebuild de `pw-world-126`, `pw-world-155` e `pw-admin-api`).** 1) Ficha do Tsuko (1.2.6): dez janelas do mesmo tamanho, ícones e textos maiores; Estatísticas só mostra, "Editar" abre o modal. 2) Equipamento: o voo 15732 com ícone e, ao passar o mouse, nível, voo simples/acelerado `+15.00`, tempo ativo 450/900, classes e nível; Inventário: o Anel de Diamante 310 com a dica completa (ornamento, durabilidade, requisitos) e, ao clicar, os blocos de equipamento. 3) Clique num slot vazio da bolsa → "Criar item" → busca por nome e categoria → Criar: o item aparece **naquele slot** (no jogo também, com o personagem online). Slot vazio do corpo: só o que veste nele; nasce na bolsa. 4) Habilidades: "Dar habilidade" filtra por classe; clique → nível → Ensinar. 5) Mascotes: "Dar mascote" com tipo/ambiente → Dar (online: o mascote aparece na jaula do jogo); clicar num da jaula: dar habilidade pela busca. 6) Missões: só as ativas, com as cores do jogo; "Editar" busca nas ativas e concluídas e dá missão nova. 7) Toda ação: toast no canto que some sozinho (sem OK). Histórico 216 |
| B215 | **Commitado (`4df4376`), não publicado; falta ver em jogo (mexe no GS e no painel: rebuild de `pw-world-155`, `pw-world-126`, `pw-admin-api`).** Personagem em jogo: 1) armazém — abra o armazém no NPC, no painel edite a quantidade de um item do armazém → o número muda na janela do armazém sem relogar (e, sem ter aberto o armazém, ao abrir já vem o novo); 2) 1.2.6 com a janela de habilidades (K) aberta: descer e remover uma habilidade pelo painel → some/desce na janela; 3) janela de missões aberta: cancelar uma missão no 1.2.6 e esquecer uma concluída (as duas versões) → a lista atualiza; atalhos e configuração continuam; 4) janela Daimon na ficha: dados do Daimon vestido; "Dar EXP" → no jogo a barra de EXP do Daimon sobe (ou ele sobe de nível). Histórico 215 |
| B214 | **Local, falta ver no painel.** Editar uma arma no painel, buscar "Garra" ou "Mordaz" nos efeitos: aparece "✦ Mordaz: Aumenta seu poder de ataque físico. — habilidade Garra"; escolher põe a linha com o valor fixo do id (habilidade 205, nível 1). Na peça que não aceita (armadura, por exemplo), não aparece. Histórico 214 |
| B213 | **Local, falta ver no painel.** Editar o Punho da Luz Fixa (8292) do WB (1.2.6): o efeito 1286 mostra "Mordaz: Aumenta seu poder de ataque físico." com o selo ✦ (dica: "Habilidade do item: Garra (ID 205)") e borda dourada; na busca de efeitos, os de habilidade vêm com "✦". Histórico 213 |
| B211 | **Local, falta ver em jogo (reiniciar o `pw-world-155`: os `npcgen` são montados, o código precisa de rebuild).** 1) Mapa 1, cidade da Névoa Sombria (3546, 281, 3052; chega-se pela missão "Retornar à Pan Gu" das classes 10/11 ou pelo teleportador): o Ancião da Névoa Sombria, o Oficial, os Guardas e o Teletransportador Rayga (que leva de volta ao Vale Celestial, 161) estão lá; sem Sereias na estrutura. 2) Sereias e outros monstros de água passeiam dentro d'água; os de ar, no ar. 3) Pedra de dano por elemento na arma → o golpe normal causa dano daquele elemento e a ficha mostra o "dano adicional"; pedra de resistência → a resistência sobe. 4) Peça com rebote (Feitiço da Purificação/Infinito) → ao apanhar, às vezes limpa maldição e acelera / regenera. 5) Painel: efeitos que ainda não agem aparecem com "ainda não age no jogo". Histórico 211 |
| B210 | **Local, falta ver em jogo.** Conjuntos: vestir N peças de um conjunto (ver a dica do cliente) soma o bônus de N peças, tirar uma o desfaz; arma com habilidade de item (ex.: atordoar 5%) → de vez em quando o monstro fica atordoado no golpe normal (log `habilidade do item` só se faltar porte); peça com escala de defesa → a defesa dela sobe pela %; experiência % → abate dá mais exp; velocidade de ataque do item → golpe mais rápido. Histórico 210 |
| B209 | **Atributos confirmados em jogo (2026-10-08, Tsuko no 1.5.5).** Efeitos de equipamento (1.5.5, RT): no modal Editar item, a busca só oferece o que existe no realm e serve na peça, com o valor do id ("Acerto +118"); linha fixa travada; Def +80/MP +999 continuam; Acerto 1317 entra +118; nível de ataque (2029 = +1) e penetração aparecem na ficha (`OWN_EXT_PROP`); bota com 286 → +5% de velocidade; armadura com 332 → conjuração 3% mais curta; 831 não aparece num elmo. 1.2.6: a busca não oferece nível de ataque nem penetração. Histórico 209 |
| B208 | **Confirmado em jogo (2026-10-08): a Tsuko saiu da Caverna das Sombras pelo altar; portão §6.2 cumprido.** Falta conferir: entrar numa masmorra por portal do mundo, a recusa com o 161 desligado, e o `CheckDeny(CMD_MOVE)` (sentado/preso não deveria passar). Na Caverna das Sombras, andar até o altar do começo do mapa: a Tsuko vai ao mapa 161 (log `mundo: portal de região`, `de=169 para=161`). Sair e entrar de novo no jogo: continua no 161. Parar fora do altar não faz nada; se o 161 estiver desligado pelo painel, o cliente mostra o erro de não poder entrar (log `portal recusado`, motivo). Os portais do mundo para as masmorras também passam a funcionar quando o mapa da masmorra é servido. |
| B204 | **Local.** Editar uma arma e pôr o efeito 831 (ataque mágico máx): a ficha mostra o ataque mágico maior depois de salvar (online, com a arma vestida). Tirar o efeito volta ao valor antigo. |
| B205 | **Local.** Caverna das Sombras: matar o Cavaleiro Negro — ele não volta. Monstro do mundo aberto continua renascendo. |
| B206 | **Local.** Editar item: cada efeito mostra a descrição; "Buscar efeito" pelo texto acrescenta a linha; Furos 0–4 muda a lista de pedras; "Buscar pedra" põe a pedra no primeiro furo vazio. 5 furos = recusado. |
| B207 | **Local.** Tsuko sem arma: Aura de Aço → ícone da aura e defesa maior nela e nos aliados em volta por 30 min. |
| B199 | **Local, não publicado; mexe no link, no barramento e no GS.** Com um bilhete de troca de rosto (4411/5382/9709/26653) na bolsa, falar com o NPC de rosto e usar: a tela de rosto abre (log `troca de rosto aberta`). Cancelar → fecha, o bilhete fica. Mudar o rosto e confirmar → um bilhete some, o outro cliente vê o rosto novo; sair e entrar → o rosto novo continua (log `aparência nova gravada`). Na seleção de personagem, um personagem criado hoje pode personalizar; um antigo não. |
| B200 | **Local.** Arma com duas pedras de nível 7+ (ou armadura com pedras 5+): o outro cliente vê o efeito das pedras. |
| B201 | **Local.** Vestir uma peça "vincula ao equipar": a dica passa a dizer vinculado e não deixa vender/trocar no cliente. |
| B202 | **Local.** Amuleto HP/MP: tirar para a bolsa → recusado; vestir outro por cima → o velho some. Mesmo item de habilidade nos dois slots → recusado. Peça com reputação acima da sua → recusada (1.5.5). |
| B203 | **Local.** Personagem com passiva de defesa/mana (ex.: 2446, 1188) ou de arma (ex.: 1371 com espada): a ficha mostra a defesa, a regeneração e o dano maiores; trocar a arma de classe tira o bônus de arma. |
| B198 | **Commitado (`c541bcb`), não publicado; mexe no GS (canal) e no painel.** Personagem **em jogo** com a janela de missões do cliente aberta: no painel, janela Missões → as ativas do cliente aparecem (etapa recuada, abates). **Dar**: buscar pelo nome, clicar, Dar → a missão aparece na lista do cliente na hora (missão de etapa: escolher a etapa). **Concluir** numa missão de NPC → no cliente fica pronta; ir ao NPC e entregar. Concluir numa de conclusão direta → prêmio e some da lista. **Cancelar** (1.5.5) → some do cliente sem prêmio; no 1.2.6 → "desconectar e aplicar". **Esquecer** numa concluída → "desconectar e aplicar"; depois de entrar, a missão pode ser pega de novo no NPC. Log: `painel: missão (online|offline)`; offline com prêmio em dinheiro/EXP → `precisa_estar_online`. |
| B196 | **Commitado, não publicado; mexe no GS (canal).** Personagem **em jogo** (1.5.5), janela de habilidades do cliente aberta (K): no painel, Ensinar uma habilidade nível 3 → aparece no cliente na hora; clicar nela → nível 5 → Aplicar → o cliente mostra 5; nível 2 → Aplicar → volta a 2 (o atalho dela sai da barra); Remover → some. No **1.2.6**, subir/ensinar funcionam em jogo; descer/remover pedem "Desconectar e aplicar". Offline: tudo grava e aparece ao entrar. Log: `painel: habilidade editada (online|offline)`. |
| B195 | **Commitado, não publicado (só painel).** Abrir um personagem → janela **Habilidades**: os ícones das habilidades aprendidas, com o nível no canto; passar o mouse mostra nome (como no cliente BR) e nível/máximo. Comparar com a janela de habilidades do cliente (tecla K). |
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

**Achado do B251/B253 (a conferir em jogo):** no 1.2.6 o carimbo do `region.clt` do cliente difere do `region.sev`
do realm em **a27, a31 e nc01**, e o do `precinct` em **a12, a27 e nc01**; na a27 e na a31 até a sequência de portais
difere. O cliente compara **os dois** carimbos ao receber o `INST_DATA_CHECKOUT` e recusa a instância se algum difere
(`CECWorld::CheckOutInst`, `EC_World.cpp:2389-2409`) — suspeito para essas instâncias não carregarem no 1.2.6. O B253
deixou pronto, no Estúdio, refazer o `.sev` do realm a partir do `.clt` do cliente (não aplicado).

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
- **Masmorra por grupo (`falta`, para implementar no futuro):** no original cada masmorra (instância, `cgame/gs/instance/`) é **uma cópia por grupo**: entrar pelo portal cria ou reaproveita a cópia daquele grupo, os monstros nascem nela, e quando a cópia fecha (todos saem e o prazo vence) ela é reciclada — ao entrar de novo, tudo volta ao início. Aqui cada masmorra é **um mapa único e compartilhado** (o `WORLD_TAGS` do GS): todos os jogadores e grupos dividem os mesmos monstros, e o que não renasce (`bAutoRevive` falso, B205) só volta ao recarregar o mapa (desligar/ligar pelo painel, B183) ou reiniciar o GS. Implementar: cópias de `WorldInstance` por grupo (ou por jogador sem grupo) para os mapas `instancia: true` do catálogo, criadas na entrada pelo portal (B208), com prazo e reciclagem conferidos em `instance/*_manager.cpp`, e o roteamento de mensagens por cópia.
- **Efeitos de equipamento — B209 (fatias 1–3, atributos confirmados em jogo) e B210** (conjuntos,
  habilidade do item no golpe, penetração/resiliência no combate, escala da peça, dano máximo,
  experiência %, regeneração %, velocidade de ataque; B211 pedras por elemento, rebote, essências de
  escala, alcance; histórico 209–211). **Falta (43 ids, o painel avisa):** habilidade de item durativa e
  as sem roteiro, poder da alma, durabilidade/requisito/aleatório da geração, `query_other_property`;
  velocidade de montaria; `_vigour_base`.
  Diagnóstico original abaixo.
- **Efeitos de equipamento (diagnóstico de 2026-10-08, teste da RT no 1.5.5):** dos 2.911 ids de
  `EQUIPMENT_ADDON`, o GS soma 925 ao vestir, 210 são refino e 493 essência (B204); **1.283 ids de 180
  tratadores não fazem nada** (`addons_sem_porte` no log) — entre eles `enhance_speed_addon` (286–290,
  velocidade), `reduce_cast_time_addon` (332…, conjuração), os de conjunto (`SET_ADDON_MACRO`),
  penetração, resiliência, vigor, `item_skill_addon` (72). O `OWN_EXT_PROP` do 1.5.5 manda **zero fixo** em
  nível de ataque/defesa, crítico, penetração, resiliência e vigor (`versions/v155/mod.rs`), então mesmo os
  somados (nível de ataque) não aparecem na ficha. **Cada id tem valor próprio** no `param1..3` do
  `EQUIPMENT_ADDON`, às vezes em float (286 = 0,05 → 5%, `arg_addon<PERCENT>`; 1317 = 118 fixo) e
  muitos servem só a uma família (o 831 é de arma; posto num elmo não age nem no original). O painel deixava
  digitar qualquer valor e qualquer id em qualquer peça.
- **Faltas do B199–B203:** estado `PLAYER_STATE_COSMETIC` (andar/lutar com a tela de rosto aberta);
  ids 201–203 e o erro 96 no cliente 1.2.6; vincular na entrada o que já estava no corpo e recusas de
  vender/soltar/trocar vinculado no servidor; cor de moda nos bits altos do id; passivas comuns sem
  modelo (dano por elemento, `Addskilldamage`, redução por distância, invisibilidade, `Incrange`,
  `Incfeather`, `EVENT_ENTER`).
- **Painel (B165–B182):** ver em jogo os roteiros B175–B179 e B182; depois E5 (inventário,
  habilidades, missões, posição, aparência, mascotes — cada um com o S2C que atualiza a ficha
  online, senão só offline), E6 subsistemas. E8 (Templates) testado no B217; **E9 fechada no B218 — a reforma
  terminou no código**; falta ver em jogo E4–E6 e E8 e publicar. Fila depois da reforma (B218): editores de
  `elements`/`tasks`/`npcgen` e recursos do cliente pelo contrato de `docs/admin/EXTENSOES_E_EDITORES_FUTUROS.md`
  (primeiro passo: escritor com ida e volta byte a byte; volume gravável no Docker); conferir textos e nomes de
  mapa do 1.2.6 no cliente 1.2.6; recarga dos dados sem reiniciar; "aplicar aos existentes" (fora por decisão de
  2026-10-11); intermitentes `nao_da_para_entrar_num_grupo_sem_convite` e `o_guia_selvagem…1177`.
- **Recursos do cliente, versionamento e atualizador (B219, só plano):**
  `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` — cliente de referência importado uma vez por
  linha (`155br`, `126`), registro de alterações no banco (objetos por SHA-256, diferença por campo nos
  `.data`), atualizador próprio no lugar do `.cup` (decisão do Murillo, 2026-10-12), fases F1–F7 e 5
  decisões abertas. Fatos novos: `elements`/`tasks` têm o **mesmo MD5** no servidor e no cliente; pelo
  fonte o cliente lê **o pacote antes do disco** (`AFileImage.cpp:87-135`; falta conferir no binário);
  o `create-patch` do `pw-patch-tool` é simulado.
  **B229 (revisão, §7 do documento):** cliente **só com arquivos soltos** pela chave `sepfile` da linha de
  comando (`ElementClient.cpp:1263-1276`; presente nos binários BR 2569 e 1.2.6) — falta o teste em jogo
  (§7.5); launcher pw-universal com modos **Jogar** e **Estúdio** (editor no Windows, mesmos crates Rust) e
  painel web para governança do conteúdo; fases G0–G6.
  **B230 (decisão, §8):** 3º teste do Murillo — cliente sem `.pck` e BR com pacotes iguais em memória e FPS;
  linha **toda solta**, instalada por **zip** da `element` (nomes em bytes GBK; o launcher extrai pela ACP),
  atualizações por arquivo; editores no **painel web**, launcher só Jogar + canal de teste, modelos pelo
  Blender. Próximos passos P1–P6 (P1 = serviço `pw-recursos`).
  **B231 (análise):** Estúdio no painel web — `docs/admin/ESTUDIO_DO_PAINEL.md`: cliente inteiro no `pw-recursos`;
  inventário (231 tabelas no v156, 30 serviços de NPC, `TALK_PROC`, habilidades são código no `elementskill.dll`);
  telas guiadas por catálogo sobre o layout; three.js; Svelte; fases S1–S8; editor de mapa completo (objetos, rios,
  relevo) documentado como futuro (§8, níveis M1–M5).
  **B232:** 1.2.6 sem pacotes confirmado em jogo (as duas versões rodam soltas com `sepfile`); pacote de instalação
  por pasta em `.tar.zst` com nomes em bytes GBK e manifesto assinado, gerado da instalação original com `.pck`
  (`RECURSOS…` §8.5–8.6); editor visual da Loja Gold no Estúdio (§4.6).
  **B233 (P1 feito, falta o jogo):** `pw-recursos` (spec 07): importar → objetos + pacotes `.tar.zst` por pasta (tar
  GNU com nome GBK) + manifesto e `versao.json` assinados (Ed25519, chave criada sozinha num volume só dele) + HTTP
  8100 com Range; `conferir` é o núcleo do launcher. 1.2.6: 61.234 arquivos, pacotes 2,8 GB, 4,5 min; 1.5.5: 327.388
  arquivos (18,9 GB), pacotes 8,5 GB, 12,7 min. Sem instaladores avulsos nem sobras do `data` do 1.2.6 (inclui o `npcgen.data`, que é do servidor). Faltam: o Murillo
  rodar as duas pastas extraídas (feito: funcionaram); **P2 launcher**; envio do cliente original pelo painel (P3) para o servidor externo.
  **B234 (P2):** núcleo `crates/pw-atualizador` (instalar retomável, verificar, reparar e atualizar por objeto, jogar) e
  app Tauri `tools/pw-launcher`; rotas `/versoes` e `objetos/<sha>` no `pw-recursos`; `enable/disablemoremem*.bat` fora
  do 1.5.5. Confirmado em jogo.
  **B235 (P3 + usabilidade):** launcher com pasta por versão, diálogo do Windows, adotar instalação existente,
  checagem leve de atualização; `pw-recursos` com API administrativa 8101 (só rede interna, segredo) — rascunho,
  trava por caminho, publicar N+1 assinado, reverter; versão N reaproveita os pacotes da 0 (o resto por objeto);
  página "Recursos do cliente" no painel. Falta: o Murillo testar; publicar `servidor`/`ambos` direto no realm;
  canal de teste GM e autoatualização do launcher; envio do cliente original pelo painel; P4 (editor de elements).
  **B237 (P4 = Estúdio S1):** escritor do `elements` com ida e volta byte a byte (`pw_data_loader::elements_editavel`),
  espaços de id do `add_id_index` (17 tabelas do v156 sem espaço conhecido: sem registro novo); API e página Estúdio;
  publicar grava `servidor`/`ambos` no `config` do realm. Falta: o Murillo testar em jogo; "aguarda reinício";
  diferenças lado a lado; S2 (telas próprias, índice de referências).
  **B238 (S2, 1ª parte):** índice de referências `specs/estudio/referencias.json` (2.813 campos com evidência do
  `ExpTypes.h` e dos dados; 2.007 duvidosos) — referência com nome/link, "usado por", remoção pelo índice, busca por
  espaço, campos repetidos em tabela. Falta: telas próprias com rótulos em português e ícones.
  **B239 (S2, 2ª parte):** `specs/estudio/rotulos.json` (rótulos traduzidos do `ExpTypes.h`, grupos da ficha) e
  ícones (`file_icon` → rota de ícones do painel) na ficha, listas, busca e referências. Falta do S2: regras próprias por
  área (categoria em modal, 3D, onde nasce); próximo: S3 (NPCs, serviços, conversa, receitas).
  **B240 (S3, 1ª parte):** `TALK_PROC` estruturado (`pw_data_loader::conversas`, ida e volta byte a byte), API e editor
  de conversas no painel, funções do `enum SERVICE_TYPE` com nome. Falta do S3: rótulos de NPCs, serviços e receitas.
  **B241 (S3, 2ª parte):** rótulos e grupos de NPCs, serviços e receitas. S3 no nível do S2; próximo: S4 (mapas:
  minimapa interativo, `npcgen`, `region`/`precinct`, teleportes, `task_npc`).
  **B242 (S4, 1ª parte):** aba Mapas no Estúdio — lista do `instance.txt`, peças do minimapa (DXT → PNG), criaturas do
  `npcgen` do realm com nome e link. Falta: editar o `npcgen`, cidades/renascer, teleportes, `task_npc`.
  **B243 (S4, 2ª parte):** `npcgen` editável (ida e volta idêntica nos 117 arquivos), áreas de criatura e de recurso
  editadas no mapa num rascunho só do servidor, publicadas no realm e revertíveis. Falta: `region`/`precinct`,
  teleportes, `task_npc`, objetos dinâmicos/controladores, altura do chão pelo GS.
  **B244 (S4, 3ª parte):** distritos (`precinct.sev` + nomes do `precinct.clt`) e portais (`region.sev`) no mapa;
  editáveis o ponto de renascer e o ponto de chegada (o resto exige editar o `.clt` do cliente junto).
  **B245 (S4, 4ª parte):** destinos de teleporte (`world_targets.sev`) no mapa, com transportadoras e NPCs; posição
  editável. Corrigido o nome do último campo do arquivo (domínio, não ordem). Falta: `task_npc`.
  **B246 (S4, 5ª parte):** rastreador de NPCs (`data\task_npc.data`, do cliente: o `gs` não o lê) — camada no mapa e
  botão no editor de área; primeiro item de destino `cliente` do Estúdio.
  **B247 (S4, 6ª parte):** objetos dinâmicos e controladores do `npcgen` editáveis (horários, ligar ao subir, quem cita).
  **B248 (S4, 7ª parte):** altura do chão pelos `.hmap` do realm no "Posicionar pelo clique".
  **B249:** "aguarda reinício" (última publicação de servidor × partida do GS) e reiniciar o realm pelo painel.
  **B250 (S4, 8ª parte):** modelo dos objetos dinâmicos (`configs\DynamicObjects.data`). Do S4 sobra criar distritos,
  portais e destinos novos (com os arquivos do cliente).

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
| 196 | 10-07 | painel E6: habilidades — ensinar, subir, descer, remover; online pelo `LEARN_SKILL` (um por nível; 0 remove), 1.2.6 descer/remover só offline |
| 197 | 10-07 | painel E6: mascotes — ver a jaula (ícone do `PET_ESSENCE`), editar o `pet_data` (limites do formato), libertar; online `PET_ROOM`/`FREE_PET` |
| 208 | 10-08 | portais de região: C2S 86 e caixas do `region.sev` — sair da masmorra pelo altar; masmorra por grupo anotada como falta |
| 204 | 10-08 | efeito de essência pelo painel aplicado à essência (o 831 conta) |
| 205 | 10-08 | `bAutoRevive` do `npcgen`: masmorra não renasce (Caverna das Sombras) |
| 206 | 10-08 | painel: descrição/busca de efeitos, busca de pedras, Furos 0–4; limite de 4 furos (`MAX_SOCKET_COUNT`) |
| 207 | 10-08 | Aura de Aço: `filter_Ironshield` portado |
| 199 | 10-08 | troca de rosto em jogo gravada: serviço 24, sessão cosmética, `SetCustomData` pelo GS (barramento 100/101), regra dos 2 dias fora do jogo; `DROP_TYPE_USE` 11 corrigido |
| 200 | 10-08 | brilho das pedras para quem vê: `_modify_mask << 16` no id do equipamento (não há brilho de refino no cliente) |
| 201 | 10-08 | vínculo: `proc_type` da instância no `OWN_ITEM_INFO`; vestir `BIND2` vincula |
| 202 | 10-08 | vestir: amuleto HP/MP não sai e é destruído ao trocar; habilidade dinâmica repetida; reputação |
| 203 | 10-08 | passivas comuns `EVENT_RESET`/`EVENT_WIELD` (catálogos com `efeito_passivo`) |
| 198 | 10-07 | painel E6: missões — ver, dar livre (`NEW`), concluir forçando o sucesso (`FINISHED`/`COMPLETE`), cancelar apagando (`GIVE_UP`; 1.2.6 só offline), esquecer a conclusão (offline); resposta do canal até 64 KiB |
| 242 | 10-14 | Estúdio S4: mapa interativo (instance.txt, minimapa DXT→PNG, criaturas do npcgen), só leitura |
| 243 | 10-15 | Estúdio S4: editar o npcgen no mapa (escritor ida e volta, rascunho só do servidor, publicar/reverter no realm) |
| 244 | 10-15 | Estúdio S4: distritos (precinct) e portais (region) no mapa; ponto de renascer e de chegada editáveis |
| 245 | 10-15 | Estúdio S4: teleportes (world_targets.sev) no mapa, com transportadoras; posição editável |
| 246 | 10-15 | Estúdio S4: rastreador de NPCs (task_npc.data do cliente) no mapa e no editor de área |
| 247 | 10-15 | Estúdio S4: objetos dinâmicos e controladores do npcgen (horários, quem cita) |
| 248 | 10-15 | Estúdio S4: altura do chão (.hmap do realm) no "Posicionar pelo clique" |
| 249 | 10-17 | Painel: "aguarda reinício" e reiniciar o realm (GS salva todos e encerra; Docker sobe de novo) |
| 250 | 10-18 | Estúdio S4: modelo dos objetos dinâmicos (DynamicObjects.data do cliente) |
| 251 | 10-18 | Estúdio S4: precinct.clt/region.clt do cliente (104 idênticos); distritos e portais novos em par |
| 252 | 10-18 | Estúdio S4: destinos de teleporte em par com o world_targets.txt do cliente (nome, criar, remover) |
| 253 | 10-18 | Estúdio S4: alinhamento cliente × servidor (.sev gerado do .clt); 14 pares diferem, a61 do 1.5.5 inclusive |
| 254 | 10-18 | Estúdio S7: Loja Gold (gshop*.data editável, item "ambos": cliente e realm juntos) |
| 255 | 10-18 | Estúdio S7: navegador de missões (tasks.data do realm, só leitura) |
| 256 | 10-18 | Estúdio S7: editar missões no lugar (campos de tamanho fixo; tasks.data "ambos") |
| 257 | 10-18 | Estúdio S7: categorias da Loja Gold (renomear, nova no fim, tirar a última sem uso) |
| 258 | 10-18 | Estúdio S7: comportamento dos monstros (aipolicy.data, só leitura) |
| 259 | 10-18 | Estúdio S7: editar o aipolicy no lugar (flags e parâmetros de tamanho fixo; item "servidor") |
| 260 | 10-18 | Estúdio S7: textos do cliente (fixed_msg, item_desc, item_ext_desc; item "cliente") |
| 261 | 10-18 | Estúdio S7: aipolicy estrutural — trocar fala, duplicar e remover gatilho |
| 262 | 10-18 | Estúdio S7: textos longos das missões (tamanho variável, tabela de deslocamentos corrigida) |
| 241 | 10-14 | Estúdio S3: rótulos e grupos de NPCs, 30 serviços e receitas |
| 240 | 10-14 | Estúdio S3: conversas (TALK_PROC) estruturadas, editor de janelas/opções, funções com nome |
| 239 | 10-14 | Estúdio S2: rótulos em português do ExpTypes.h, grupos da ficha, ícones |
| 238 | 10-14 | Estúdio S2: índice de referências do elements (ExpTypes.h + dados), usado por, busca por espaço, listas |
| 237 | 10-14 | Estúdio S1: escritor do elements (ida e volta), ids por espaço, editor no painel; publicar grava no realm |
| 236 | 10-14 | teste intermitente do Guerreiro 126: era a espera da morte do monstro sem prazo real (carga na máquina); só teste |
| 235 | 10-14 | launcher com pasta por versão e diálogo; P3: rascunho, publicar e reverter (API 8101 + página no painel) |
| 234 | 10-14 | P2: núcleo do launcher `pw-atualizador`, app Tauri `pw-launcher`, rotas `/versoes` e objetos; `.bat` de memória fora do 1.5.5 |
| 233 | 10-14 | P1: serviço `pw-recursos` (8100), versão 0 do 1.2.6 e do 1.5.5 importada, pacotes `.tar.zst` assinados, conferência por HTTP |
| 232 | 10-14 | 1.2.6 solto confirmado; pacote `.tar.zst` por pasta e carga no servidor; editor da Loja Gold; commits |
| 231 | 10-14 | análise do Estúdio no painel (inventário, viabilidade, arquitetura, tecnologia, S1–S7); sem código |
| 230 | 10-14 | 3º teste: cliente solto igual ao com pacotes; decisão: zip da `element`, nomes GBK, editores no painel web, passos P1–P6; sem código |
| 229 | 10-13 | plano revisto: cliente só com soltos (`sepfile`), assinatura Ed25519 no `pw-recursos`, launcher com Jogar e Estúdio; sem código |
| 228 | 10-12 | teste do empurrão de ar no mapa do espaço |
| 227 | 10-12 | auras (`Auracurse`/`Aurabless`) em jogador e mascote |
| 226 | 10-12 | amuleto de dano: reforço físico/mágico e gasto por uso (slot 17) |
| 225 | 10-12 | dano antes do grau (`Resultado::antes_do_grau`) no transporte de dano da planta |
| 224 | 10-12 | equipamento produzido sem bloco (durabilidade 0, classe vermelha): bolsa gravada antes dos avisos; nome do fabricante até 20 B no 1.2.6 |
| 223 | 10-12 | (interrompido) força da alma dos addons; reforço de mascote conferido como zero no original |
| 222 | 10-12 | planta invocada (Místico, Feiticeira de seita, classes novas): nascer, automática/passiva, grupo, vida, explosão, avisos 293–295; habilidade de mascote em área; Incrementalhpgen/Transportdamagetoleader/Removeaggro/Pullover2; força da alma; empurrão de água/ar |
| 221 | 10-12 | Bárbaro 1.2.6 (Força Divina, Sangue Fervente, Rugido do Tigre, Presas Venenosas, Fúria Bestial, Terra Dividida, Ira do Tigre, Mordida/Desafio) e Feiticeira 1.5.5 de seita; chi do Devastar no PvP; forma de classe = 1 no 1.2.6 |
| 220 | 10-12 | habilidade instantânea sem parar (C2S 80 → só o 178); Traje de Flores (`Swiftform`) e 7 filtros da Feiticeira; estados visíveis traduzidos no 1.2.6; `PLAYER_DROP_ITEM` da versão na produção/refino/renascer |
| 219 | 10-12 | plano: recursos do cliente (cliente de referência, formatos de modelo/animação), registro de alterações e atualizador próprio no lugar do `.cup`; sem código |
| 218 | 10-11 | painel E9: fechamento da reforma — extensões para os 6 editores futuros, manual do GM, interface revista (ícones, tema claro, nomes de mapa, Daimon só no 1.5.5), semeio sem a Poção 1801, restos removidos |
| 217 | 10-11 | painel E8: Templates por realm (estatísticas, bolsa, corpo, habilidades) lidos na criação; bandeja das janelas recolhidas |
| 216 | 10-11 | painel: reforma visual da ficha (janelas iguais, modais, toast); criar item no slot, dar mascote, catálogos por categoria/classe/tipo; voo do 1.2.6 sem nome/ícone (tabela de item) e equipamento sem octetos (bloco do molde) |
| 215 | 10-11 | painel: E6 fechada — Daimon (ficha e dar EXP) e edições em jogo (armazém, habilidade no 1.2.6, esquecer/cancelar missão pelo reenvio do `TASK_DATA`) |
| 214 | 10-10 | painel: busca de efeitos acha habilidade do item pelo texto e pelo nome (cache por realm do GS) |
| 213 | 10-10 | painel: efeito de habilidade do item (tipo 55) com a introdução do `skillstr` e selo ✦ |
| 212 | 10-10 | painel E7 (rates e mapas) confirmado em jogo no 1.2.6 (WB); sem código |
| 211 | 10-08 | `npcgen` do mapa 1 (e 5 outros) do `pwserver_155v156`: cidade da Névoa Sombria; passeio de água/ar; pedras por elemento, rebote, essências de escala, alcance; painel avisa o que não age |
| 210 | 10-08 | efeitos: conjuntos (`SUITE_ESSENCE`), habilidade do item no golpe, penetração/resiliência, escala da peça, exp %, regeneração %, velocidade de ataque |
| 209 | 10-08 | efeitos de equipamento: valor do id, família e realm na edição; `OWN_EXT_PROP` com graus; velocidade, conjuração e resistências dos addons |