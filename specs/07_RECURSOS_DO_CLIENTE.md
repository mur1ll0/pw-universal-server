# 07 — Recursos do cliente (`pw-recursos`)

> Como o servidor guarda o cliente de cada versão e o entrega ao launcher. Plano e decisões:
> `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (§3, §7.2, §7.7, §8, §8.6). Estado: **P1
> (versão 0 + pacotes + HTTP) no B233, confirmado em jogo nas duas versões; P2 (núcleo do launcher
> `pw-atualizador` + app Tauri `tools/pw-launcher`) no B234, confirmado em jogo; usabilidade do launcher e P3
> (rascunho, publicar e reverter pelo painel) no B235; Estúdio S1 (editor genérico do `elements.data`) e a gravação
> no realm ao publicar no B237**; telas próprias por área (S2+) e o envio do cliente original pelo painel faltam.

## 1. Serviço

| | |
| :--- | :--- |
| crate | `crates/pw-recursos` (bin + lib) |
| contêiner | `pw-recursos` (`docker/Dockerfile.recursos`: Rust + Python 3 + `tools/pw-pck-extract`) |
| porta | **8100**, só leitura: `GET /versoes` (lista JSON das versões com `versao.json`), `GET /atualizacao/<versao>/<resto>` (com `Range`, `ServeFile` do tower-http), `GET /atualizacao/<versao>/objetos/<sha256>` (um arquivo pelo conteúdo, zstd — reparo e atualização, B234) e `GET /saude` |
| volume de dados | `data/clientes` → `/dados` (bind) |
| chave | volume nomeado `pw_recursos_chave` → `/chave/assinatura_ed25519.key`, **montado só aqui** (o painel monta `../data` inteiro) |
| API administrativa | **8101**, sem `ports:` (só a rede interna; o painel a chama), segredo `PW_RECURSOS_SEGREDO` (de `RECURSOS_SEGREDO` no `docker/.env`) no cabeçalho `x-recursos-segredo`; sem segredo de 16+ caracteres fica desligada. Rotas no §6 |
| banco | `recursos_linhas`, `recursos_base`, `recursos_alteracoes`, `recursos_alteracao_itens`, `recursos_versoes` (`scripts/2026_10_14_recursos_do_cliente.sql`; repositório `pw_storage::RecursosRepository`) |

Comandos (`pw-recursos <comando>`; no Docker, `docker compose run --rm … pw-recursos <comando>`):

| comando | faz |
| :--- | :--- |
| `servir` (padrão) | HTTP na 8100 |
| `importar --versao V --cliente <element original com .pck> [--refazer] [--nivel 12]` | monta a versão 0 (§3) |
| `conferir --origem <http://host:8100 \| pasta> --versao V --destino <pasta vazia> [--chave-publica hex]` | prova: assinatura, extração pela ACP, hashes (o núcleo do launcher) |
| `limpar --versao V` | apaga de `objetos/` todo objeto que nenhum registro da versão usa (base + antes/depois das alterações) e os temporários; roda sozinho no fim do `importar` |
| `chave` | imprime a chave pública (cria o par se não existe) |

## 2. Nomes: bytes GBK, nunca texto

O cliente abre arquivo solto com os **bytes GBK** dos dados por `fopen` ANSI
(`AngelicaFile/Source/AFileImage.cpp:127`); o Windows converte pela ACP de cada máquina. Então:
banco (`caminho BYTEA`), tar e manifesto (`caminho_gbk` em hex) guardam os bytes do índice do
`.pck`, com separador `\` sem conversão (um 2º byte GBK pode ser 0x5C). Quem extrai calcula o
nome com `MultiByteToWideChar(CP_ACP, MB_ERR_INVALID_CHARS)` (`nomes::nome_no_disco`) — idêntico ao
`bytes.decode("mbcs")` do `pw-pck-extract` (conferido: 58/58 nomes do `configs.pck` 1.2.6, 17 não-ASCII).
Comparação de caminhos sem diferenciar maiúsculas (decodifica GBK e passa para minúsculas).

## 3. Importação da versão 0

Lê a instalação **original com `.pck`**, uma vez por versão (nunca uma cópia já extraída — os nomes
dela já estão na ACP daquela máquina):

1. Cada `*.pck` (recursivo; `.pkx` junto) pelo `pw_pck_extract.py fluxo` — saída binária
   `E u32 nome u64 dados` / `F u32 nome u32 erro` / `Z u32 total`. Entre pacotes, o primeiro vence
   (repetidos no relatório).
2. Soltos: nome Unicode → GBK (`encoding_rs`). **Se o caminho já veio de um pacote, vale o pacote**
   (o cliente abre o pacote primeiro, `AFileImage.cpp:94-104`) e o solto nem é lido. Nome sem GBK
   (ex.: `Padrão (N).ini` gravados pelo cliente nesta máquina) fica fora e vai para o relatório.
3. Excluídos (§8.6): `.pck`/`.pkx`, `*.pck.files/`, `sPCK.exe`, `pwrepack.exe/.bat`,
   `extract_pck.bat`, `update5.rar` (na raiz), `*.log`, e as pastas `logs/`, `screenshots/`,
   `NovosModels/`. Decisões do Murillo (B233): os instaladores avulsos da raiz (`360inst_wanmeigj.exe`,
   `gt_setup.exe`, `gt_updater.exe`, `dxwebsetup.exe`, `unitywebplayerdevelopment.exe`) e os
   `enablemoremem`/`disablemoremem_vista_win7_32bit.bat` (mexem no `increaseUserVa` do Windows por `bcdedit`); em `data\`, as pastas
   `BCK`, `Data Export`, `Editados`, `tasks`, os `.rar`/`.zip`, `Notas.txt` e o `npcgen.data` — arquivo do
   servidor: nenhum `.cpp` do cliente o abre (a única ocorrência é um campo, `elementdataman.cpp:2073`).
4. Nossos: `userdata\server\serverlist.txt` (UTF-16LE com BOM) ganha a linha de grupo
   `pw-universal` se a 1ª linha não vazia tiver tabulação — versões ≥ 1.5 (`EC_ServerList.cpp:146-157`;
   o 1.2.6 dispensa, §8.5); `elementclient.bat` ganha ` sepfile`.
5. Objetos: `<dados>/<versao>/objetos/<aa>/<sha256>.zst` (zstd, imutáveis, um por conteúdo).
6. Pacotes: um `.tar.zst` por primeira pasta (minúsculas; arquivos da raiz em `raiz`), tar GNU
   próprio (`tar_gnu.rs`: nome até 100 bytes no cabeçalho, mais longo em `././@LongLink`; mtime 0),
   zstd nível 12 multinúcleo com checksum.
7. Manifesto + `versao.json`, assinados (§4); relatório em `<dados>/<versao>/importacao_0.json`;
   banco numa transação (`recursos_linhas`, `recursos_base`, `recursos_versoes` nº 0).
   Reimportar exige `--refazer`, e só se não houver versão > 0 nem alteração. Ao fim, `limpar`: o que saiu da
   versão (exclusões novas, arquivo trocado por um nosso) não fica no servidor.

## 4. Atualização por HTTP

```text
/atualizacao/<versao>/versao.json (+.sig)            {formato, versao_cliente, atual, minima, manifesto, manifesto_sha256}
/atualizacao/<versao>/<n>/manifesto.json (+.sig)     {formato, versao_cliente, numero, gerado_em, pacotes[], arquivos[]}
/atualizacao/<versao>/<n>/pacotes/<pasta>.tar.zst    pacotes[]: {nome, arquivo, sha256, tamanho, tamanho_tar, arquivos}
                                                     arquivos[]: {caminho_gbk (hex), sha256, tamanho, pacote}
```

`.sig` = Ed25519 (64 bytes em hex) dos bytes exatos do `.json`. A chave nasce no 1º uso (decisão
B233, sem passo manual); a pública vai embutida no launcher. Perder a chave = launchers antigos não
aceitam mais atualização → **fazer backup do volume `pw_recursos_chave`** (`pw-recursos chave`
mostra a pública). O `versao.json` é gravado por último: só aparece quando a versão está completa.

## 5. Launcher (P2, B234)

| peça | onde | faz |
| :--- | :--- | :--- |
| núcleo | `crates/pw-atualizador` (lib, sem banco nem axum; também usado pelo `pw-recursos conferir`) | `Servidor::versoes`, `versao_remota` (assinaturas + SHA-256 do manifesto), `instalar`, `verificar`, `reparar`, `atualizar`, `jogar`, `conferir_range` |
| app | `tools/pw-launcher` (Tauri 2 + `tauri-plugin-dialog`, fora do workspace; `cargo build --release` na pasta) | janela `ui/index.html`; `pw-launcher.json` ao lado do `.exe` guarda o servidor, a última versão escolhida e **uma pasta por versão** (`pastas`); chave pública embutida de `chave_publica.txt` (trocar ao publicar com outra chave) |

Tela (B235): ao abrir, busca as versões no servidor (abas) e mostra a situação da escolhida, lendo só o
`versao.json` (`Servidor::versao_anunciada`, sem baixar o manifesto). Conforme a situação:

| situação | botões |
| :--- | :--- |
| sem pasta / pasta sumiu / pasta sem o jogo | **Baixar o jogo** (janela de pastas do Windows; instala na escolhida se estiver vazia, senão em `PW Universal <versao>` dentro dela) e **Já tenho instalado** (escolhe a pasta `element`, ou a pasta acima dela; adota: grava manifesto e estado e repara o que divergir — `cliente::adotar`) |
| download interrompido | **Continuar download** (retoma) e escolher outra pasta |
| instalado | **Jogar** (ou **Atualizar** + "Jogar assim mesmo" quando o servidor tem versão nova), **Verificar arquivos**, **Abrir pasta** (Explorer), **Trocar pasta** |

O endereço do servidor fica em "Configurações" (recolhido).

Na pasta do cliente (`element`), o launcher guarda só `.pw-atualizacao/`: `estado.json` (versão do
cliente, número aplicado ou `null` durante a instalação, pacotes já extraídos), `manifesto.json` (o
aplicado), `<pacote>.parcial` (download em curso) e `objetos/` (atualização em curso).

- **Instalar:** pasta vazia (ou instalação interrompida do mesmo manifesto); recusa se a ACP não converte
  algum nome (§7.7); por pacote: baixa para `.parcial` retomando com `Range`, confere tamanho e SHA-256,
  extrai conferindo cada arquivo contra o manifesto, marca pronto. No fim confere tamanhos e grava o estado.
- **Verificar:** rápido (existe e tem o tamanho) ou completo (SHA-256), em paralelo.
- **Reparar:** verificação completa e cada divergente baixado de `objetos/<sha>`.
- **Atualizar:** se `versao.json` aponta outro manifesto, baixa só os objetos novos/trocados para
  `.pw-atualizacao/objetos/`, depois grava, apaga os removidos e só então troca manifesto e estado.
- **Jogar:** executa a linha do `elementclient.bat` da versão (traz o `sepfile`) na pasta do cliente.
- **Instalar a versão N > 0:** os pacotes continuam os da versão em que nasceram (`Pacote::numero`); o que
  o pacote traz e a versão não usa mais (removido ou trocado) é pulado, e o que falta vem por objeto.

## 6. Governança: rascunho, publicar, reverter (P3, B235)

Motor em `crates/pw-recursos/src/governanca.rs`, API em `admin.rs` (8101), repasse no painel em
`web-admin/backend/painel/recursos.py` (`/api/recursos/…`, sessão GM + CSRF; o autor é o GM da sessão)
e tela "Recursos do cliente" (grupo Geral; `static/recursos.js`).

| rota (`/admin/…` no pw-recursos; `/api/recursos/…` no painel) | faz |
| :--- | :--- |
| `GET versoes` | `[{versao, atual, minima}]` |
| `GET {v}/alteracoes?limite=` | histórico (rascunhos, publicadas com a versão, descartadas), com os itens |
| `GET {v}/arquivos?busca=` / `GET {v}/arquivo?caminho=` | busca na versão publicada / baixa o conteúdo atual |
| `POST {v}/rascunhos {descricao, operacao_id}` | cria (idempotente por `operacao_id`) |
| `PUT {v}/rascunhos/{id}/arquivo?caminho=&destino=` (corpo = arquivo) | `substituir` ou `adicionar`; recusa conteúdo igual ao publicado |
| `DELETE …/arquivo?caminho=` · `POST …/remover?caminho=` · `POST …/descartar` | tira do rascunho · marca remoção · descarta |
| `POST {v}/publicar {rascunhos, minima}` | gera a versão N+1 assinada |
| `POST {v}/alteracoes/{id}/reverter {operacao_id}` | rascunho com o inverso de uma publicada |

Regras: o caminho é resolvido contra o manifesto publicado sem diferenciar maiúsculas (vale o byte que já
existe); **um caminho só em um rascunho por vez** (409); ao publicar, cada item confere que o arquivo ainda
está como estava quando entrou no rascunho; dois GMs publicando ao mesmo tempo: a versão anterior tem de
ser N (trava `pg_advisory_xact_lock` + checagem), um perde. Publicar grava `atualizacao/<N+1>/manifesto.json`,
registra no banco e só então troca o `versao.json` (gravação atômica, `.sig` antes). `minima` (opcional)
obriga o launcher a atualizar. O envio de arquivo no painel aceita até 512 MiB (`LimitarEntrada` abre
exceção só para essa rota; o resto continua 8 KiB).

**Gravação no realm (B237):** item de destino `servidor`/`ambos` publicado é gravado (troca atômica) em cada
pasta de `PW_RECURSOS_REALMS` da versão (`{"1.2.6": ["/realms/realm_126"], "1.5.5": ["/realms/realm_155"]}`,
montadas de `data/realm_*/config`); o nome do item diz onde (`governanca::caminho_no_realm`, B243):
`data\<arquivo>` → `<arquivo>` na raiz do config, `servidor\<pasta>\…\<arquivo>` → `<pasta>/…/<arquivo>` (até 4
partes, sem `..`, `:` nem vazias). Item `servidor` só é gravado se o arquivo do realm ainda tem o SHA do
`sha256_antes` (senão a publicação falha: mudou por fora); `servidor` não entra no manifesto do cliente, e o
"atual" dele para reverter é o arquivo do realm. Remoção não apaga nada do realm. A resposta do publicar lista os
`aplicados`. **O GS só lê esses arquivos ao subir: reinicie o realm** — o Estúdio mostra "aguarda reinício" e
reinicia pelo painel (B249, spec 06; `GET /admin/{v}/servidor/ultima` dá a última versão que gravou no realm).

## 6b. Estúdio, fase S1 (B237)

Motor `crates/pw-recursos/src/estudio.rs` sobre `pw_data_loader::elements_editavel` (spec 03 §3.1); painel
`/api/realms/{realm}/estudio/…` (a versão vem de `realms.version`) e a página "Estúdio" do grupo do realm
(`static/estudio.js`).

| rota (`/admin/{v}/estudio/…`) | faz |
| :--- | :--- |
| `GET tabelas?rascunho=` | tabelas do arquivo (registros, espaço de id) — do rascunho, ou do publicado |
| `GET tabelas/{T}?busca=&pagina=&por_pagina=&rascunho=` | registros com id e `Name` (busca por id exato ou nome) |
| `GET tabelas/{T}/{i}?rascunho=` | a ficha: campos com tipo, tamanho, deslocamento e valor (`string` GBK sai legível) |
| `PUT tabelas/{T}/{i}?rascunho=` `{campos}` | edita; recusa tipo errado, texto que não cabe, campo inexistente |
| `POST tabelas/{T}/{i}/clonar?rascunho=` | registro novo com o próximo id do espaço |
| `DELETE tabelas/{T}/{i}?rascunho=` | remove; recusado se algum campo do índice de referências (ou duvidoso) de outro registro cita o id |
| `GET espacos/{E}?busca=&limite=&rascunho=` | registros do espaço com id exato ou nome contendo a busca (seletor de referência) |

Cada edição grava o arquivo inteiro como objeto (zstd 3) e o item `data\elements.data` (destino `ambos`) do
rascunho, com `resumo = {campos: [{tabela, id, campo, antes, depois}], registros: [{tabela, id, acao, base}]}` —
o "antes" é sempre o do publicado; voltar ao valor publicado tira a linha; arquivo igual ao publicado tira o item.
Uma edição por rascunho de cada vez (trava em memória); os arquivos lidos ficam em cache pelo SHA-256.

**Índice de referências (S2, B238):** `specs/estudio/referencias.json`, gerado por
`specs/estudio/gerar_referencias.py` com **duas evidências por campo** — o membro do `ExpTypes.h` do cliente 1.5.5
(achatado como o layout: `nome_i_membro`) tem nome de id ou comentário que diz "ID", e ≥ 95% dos valores não nulos
dos `elements` de `realm_126` e `realm_155` existem como id no espaço. Pela cobertura só `ESSENCE`/`ADDON` (ids
altos); `RECIPE`/`TALK` exigem a palavra (配方, 对话) ou destino declarado com evidência (forja: `NPC_MAKE_SERVICE`
`pages_N_id_goods_N` → `RECIPE`); ficam de fora estratégia, quantidade, nível, classe, máscara, controlador de mapa,
habilidade e missão (não são registros do `elements`). Resultado: **2.813 campos em 70 tabelas** (`ESSENCE` 2.325,
`RECIPE` 261, `ADDON` 225, `TALK` 2); **2.007 duvidosos** com o motivo. A ficha traz `cita`, `referencia`
(`{tabela, indice, id, nome}` ou `{ausente}`) e `usado_por` (até 100); remover bloqueia por índice + duvidosos. Na
tela, campos repetidos viram tabela e a referência tem link e busca embutida.

**Rótulos, grupos e ícones (S2, B239):** `specs/estudio/rotulos.json` — rótulo em português de cada campo
(chave = nome com os índices trocados por `N`), traduzido do comentário do `ExpTypes.h`, e os grupos da ficha
(Identificação, Requisitos, Combate, Comércio, Durabilidade e fabricação, Efeitos, Drops, Comportamento,
Habilidades, Visual; o resto em "Outros", com o nome técnico — campos sem comentário no fonte não ganham
rótulo inventado). A ficha traz `rotulo`/`grupo` por campo e `icone` (o `file_icon` em hex GBK), também nas
listas, na busca e nas referências; o painel recorta pela rota de ícones (`/api/icones/m/<hex>.png`, B188).
Listas grandes mostram só as linhas preenchidas (+2 vazias ao editar) e "mostrar todas".

**Conversas (S3, B240):** `GET/POST conversas` (lista, nova vazia ou `?base=` cópia, id = maior do espaço TALK + 1),
`GET/PUT/DELETE conversas/{i}` (árvore inteira; validação: ids de janela únicos e sem o bit alto, pai existente
ou -1, opção para janela existente ou função conhecida, textos fixos até 63 caracteres; texto igual mantém as
unidades originais; remover recusado se um serviço cita), `GET funcoes` (`specs/estudio/funcoes_de_conversa.json`,
o `enum SERVICE_TYPE` do `ExpTypes.h:36-102`: `NPC_TALK = 0x80000000`… 33 códigos com rótulo). As conversas entram
no índice como `TALK_PROC` (espaço TALK): o `id_dialog` dos serviços de conversa/transporte vira link. No painel,
`TALK_PROC` aparece na lista de tabelas e abre o editor (janelas com texto, origem e opções → janela ou função);
o salvar da conversa aceita até 256 KiB (`LimitarEntrada`, só essa rota).

**NPCs, serviços e receitas (S3, B241):** o `rotulos.json` ganhou os campos das tabelas `NPC_*` (NPC e os 30
serviços) e `RECIPE_*` (277 rótulos no total) e os grupos Serviços do NPC, Páginas, Fabricação, Teleporte e
Missões (antes de Requisitos: `id_skill_service` é serviço, não habilidade). Campo sem comentário no fonte segue
sem rótulo. O painel desenha também grupo que a lista dele não conhece, antes de "Outros".

**Mapas (S4, B242, só leitura):** `crates/pw-recursos/src/mapas.rs` e as rotas `GET mapas` (do `Configs\instance.txt`
do cliente publicado — `CECInstance::Load`, `EC_Instance.cpp:60-110`: nome, id/world tag, caminho, linhas, colunas;
o 1.5.5 tem campos a mais, pulados até o `}`), `GET mapas/{caminho}/pecas/{LLCC}.png` (`Surfaces\MiniMaps\<caminho>\LLCC.dds`,
`DlgMiniMap.cpp:1155-1168`; DXT1/3/5 decodificado e PNG, em cache) e `GET mapas/{caminho}/criaturas` (`npcgen.data` do
realm em `config/<caminho>/`; o mundo cai na raiz do config só sem `world/npcgen.data`; pontos `[tipo, modelo, x, z, renascer_s, area_x, area_z]` e o
nome de cada modelo no `elements` publicado). Conta do cliente (`DlgMiniMap.cpp:1045-1061`): 2 m por pixel, peças de
256 px, `px = (x + colunas·512)/2`, `py = (linhas·512 − z)/2`. O índice dos arquivos do cliente fica em cache pelo
SHA do manifesto. Painel: aba "Mapas" do Estúdio (`static/mapa.js`) com zoom/arrasto, peças carregadas sob demanda,
pontos por tipo em canvas do tamanho da tela e clique que abre a ficha do modelo.

**Edição do `npcgen` (S4, B243):** `pw_data_loader::npcgen_editavel` lê e grava o arquivo inteiro com ida e volta
idêntica (`npcgendata.h:32-286`, `npcgendata.cpp:62-315`; `pack(1)`, `size_t` = 4: área 59/71 bytes (v≥7), gerador
60/64 (v≥11), área de recurso 31/34 (v6)/42 (v≥7), recurso 20, objeto 19/23 (v9)/24 (v≥10), controlador 195/199
(v≥8); contagens recalculadas ao gravar) — testado nos 117 `npcgen.data` dos dois realms (versões 4, 5, 6, 10, 11).
Os campos editáveis são os das structs `AREA`/`NPCGEN`/`RESAREA`/`RES` pelo nome do fonte, só os que a versão do
arquivo tem; objetos dinâmicos e controladores passam intactos. Rotas: `GET mapas/{c}/npcgen[?rascunho]` (uma linha por
área: `[indice, tipo, x, z, ext_x, ext_z, primeiro_modelo, geradores, quantidade, nasce_ao_subir]`, tipo pelo
`iNPCType` — 0 monstro, 1 NPC, `npcgendata.h:313-317` —, nomes dos modelos e os campos da versão),
`GET|PUT|DELETE mapas/{c}/npcgen/{areas|recursos}/{i}` (`PUT {campos, filhos?}`: `filhos` substitui a lista — de 1 a
64; os novos copiam o último) e `POST …/{i}/clonar` (cópia no fim; o `dwGenID` só o editor lê). Tudo num rascunho:
item `servidor\<mapa>\npcgen.data` (`servidor\npcgen.data` só se o realm não tiver `world/npcgen.data`: a mesma ordem do GS, `manager.rs:616-621`; o `realm_126` tem os dois, idênticos, e vale o de `world/`), destino `servidor`,
`sha256_antes` = o arquivo do realm (gravado também no volume, para reverter), resumo com a lista `mudancas`; voltar
ao igual ao realm tira o item. Publicar grava no realm (o GS lê ao subir: reinicie). Painel: "Editar áreas" na aba Mapas
desenha as caixas (`vExts`/`fExt` = tamanho, metade para cada lado), clique abre a área com os geradores, "Posicionar
pelo clique", "+ gerador", salvar/clonar/remover; o `PUT` da área tem limite de 64 KiB no painel. (Até o B244 as
caixas saíam com o dobro do tamanho: a meia-medida não era dividida pelos 2 m por pixel.)

**Distritos e portais (S4, B244):** a mesma edição de arquivo do realm (`alterar_arquivo_do_mapa`, pasta do mapa pela
ordem do GS) vale para `precinct.sev` e `region.sev`. Escritores: `Distritos::gravar` (`precinct.rs`, ordem de
`CELPrecinct::Load(FILE*)`, `el_precinct.cpp:219-266`) e `RegioesEditavel` (`regioes.rs`, registros na ordem do
arquivo: região tipo 0, caixa tipo 1) — ida e volta idêntica nos 115 `precinct.sev` e 115 `region.sev` dos dois
realms (431 portais). Rotas: `GET mapas/{c}/distritos[?rascunho]` (polígono `[x, z]`, ponto de cidade, mapa do ponto,
prioridade, domínio, PK; nome, id e marcas do `Maps\<c>\precinct.clt` do cliente — formato texto de
`CELPrecinct::Load(AWScriptFile*)`, `EL_Precinct.cpp:96-213` — quando a contagem e os vértices de cada distrito batem),
`PUT mapas/{c}/distritos/{i} {campos}` (`ponto`, `mapa_do_ponto`, `prioridade`, `protege_pk` só na v7),
`GET mapas/{c}/portais` (caixas com destino, origem, nível, `pos`, `exts` meia-medida, `alvo`; regiões como
polígonos) e `PUT mapas/{c}/portais/{i} {campos: {alvo}}` — **só o ponto de chegada**: o cliente acha a caixa pela
própria `region.clt` e manda só o índice e o destino dela (`EC_World.cpp:2360-2373`), então posição, tamanho e
destino só mudam junto com o cliente. O carimbo (`dwTimeStamp`) dos dois arquivos, que o cliente confere no
`INST_DATA_CHECKOUT` (`manager.rs:236-240`), nunca muda. Painel: camadas "Distritos" (polígono tracejado, quadrado no
ponto de renascer, nomes das marcas com zoom) e "Portais" (caixa e linha até a chegada quando é no mesmo mapa); clique
abre o editor com "Posicionar pelo clique".

**Teleportes (S4, B245):** `world_targets.sev` na raiz do `config` (`manager.rs:489`), formato do exportador do
cliente (`globaldataman.cpp:793-803`: `u32 n` + `n × {i32 id; i32 world_id; f32 pos[3]; i32 domain_id}`) — o último
campo é o **domínio**, não a ordem (o leitor dizia "ordem"; corrigido, `PontoDoMundo::dominio`). `PontosDoMundo` guarda a
`lista` na ordem do arquivo e grava de volta (`gravar`, idêntico nos dois realms). Rotas: `GET mapas/{c}/teleportes`
(destinos cujo `world_id` é o id do mapa no `instance.txt`, nome do `maps\world\world_targets.txt` do cliente —
`globaldataman.cpp:450-466` —, e em `usado_por` as transportadoras `NPC_TRANSMIT_SERVICE` que levam a cada um, com
preço, nível e os NPCs que oferecem o serviço por `NPC_ESSENCE.id_transmit_service`) e
`PUT teleportes/{id} {campos: {pos}}` (item `servidor\world_targets.sev`; só a posição — o mapa e o domínio ficam). O
cliente usa a posição do `.txt` dele só para saber em que região o destino fica (`EC_World.cpp:642-650`). A edição de
arquivo do realm ficou em três camadas: `item_do_realm` (nome do item → bytes do realm e do rascunho),
`arquivo_do_mapa` (pasta do mapa pela ordem do GS) e `gravar_item_do_realm` (o miolo comum). Painel: camada
"Teleportes" (losango laranja com nome), editor com as transportadoras e link para a ficha do NPC.

**Rastreador de NPCs (S4, B246):** `data\task_npc.data` é **do cliente** — lido do disco por
`ATaskTemplMan::LoadNPCInfoFromPack` (`EC_TaskInterface.cpp:166`, `TaskTemplMan.cpp:2149-2185`) e usado por
`GetTaskNPCInfo` nos nomes clicáveis das missões (`DlgNameLink.cpp:243`); o `gs` original tem o leitor mas nunca o chama
(a cópia no `config` do realm não serve). Formato (`TaskTemplMan.cpp:43-60`, `:2067-2103`; `TaskTempl.h:71-77`):
cabeçalho `{u32 pack_size; i32 time_mark; u16 versão = 1; u16 npc_count}` e `npc_count × {u32 id; i16 x, y, z; 2 bytes de
alinhamento}` (o `NPC_INFO` fica antes do `#pragma pack(1)`: 12 bytes) — conferido: 1.5.5 BR 1.572 NPCs / 18.876 bytes,
1.2.6 765 / 9.192. **No arquivo as coordenadas vêm x, z, altura**: o cliente monta o ponto com
`Set(pInfo->x, pInfo->z, pInfo->y)` ("different sequence", `DlgNameLink.cpp:246-247`); nos dois arquivos a altura é sempre
0 e (0, 0, 0) é NPC sem posição (300 de 765 no 1.2.6, 1.057 de 1.572 no 1.5.5). O cliente só consulta o rastreador no
mapa principal (`MAJOR_MAP` = 1, `DlgWorldMap.h:13`). `pw_data_loader::task_npc::Rastreador` lê, grava idêntico (os 2
bytes de alinhamento têm lixo e são preservados) e expõe/troca a posição na ordem do mundo `[x, altura, z]` (arredonda;
recusa fora de `short`; com id repetido vale o último, como o `m_NPCInfoMap` do cliente). Rotas:
`GET mapas/{c}/rastreador` (NPCs das áreas `iNPCType` = 1 do `npcgen` do mapa, com a posição do rastreador e a distância
até a área mais perto; `usado_pelo_cliente` só no mapa 1) e `PUT rastreador/{id} {campos: {pos}}`. É o primeiro item do Estúdio
de destino **`cliente`** (`alterar_item_do_cliente`: caminho GBK exato do manifesto, `sha256_antes` = o publicado):
entra no manifesto da versão nova e o launcher baixa; não vai para o realm. Painel: camada "Rastreador" (verde perto da
área, vermelho a mais de 20 m com linha até ela) e, no editor de área de NPC, a posição do rastreador de cada gerador e
"Usar a posição da área no rastreador".

**Objetos dinâmicos e controladores do `npcgen` (S4, B247):** campos pelo nome do fonte, `pack(1)`:
objeto (`NPCGENFILEDYNOBJ`/`9`/`10`, `npcgendata.h:205-232`) `dwDynObjID`@0, `vPos`@4..16, `dir_0`/`dir_1`@16/17,
`rad`@18, `idController`@19 (v≥9), `scale`@23 (v≥10); controlador (`NPCGENFILECTRL`/`8`, `:243-282`) `id`@0,
`iControllerID`@4, `szName[128]`@8 (GBK, só leitura), `bActived`@136, `iWaitTime`@137, `iStopTime`@141,
`bActiveTimeInvalid`@145, `bStopTimeInvalid`@146, `ActiveTime_*`@147 e `StopTime_*`@171 (`NPCCTRLTIME`: ano, mês 0-11,
dia da semana 0-6 com 0 = domingo, dia, hora, minuto; -1 = qualquer), `iActiveTimeRange`@195 (v≥8). Tipo novo `u8` no
`npcgen_editavel`. Conferido nos 117 arquivos dos dois realms: 238.648 controladores e 9.862 objetos com booleanos 0/1,
horários nas faixas, ids de controlador únicos por arquivo; das 39.318 áreas com `idCtrl`, 1 cita controlador que não
existe (a27 do 1.2.6, 422). A área cita o controlador pelo `id` interno; o `iControllerID` é o número do `aipolicy`
(`npcgenerator.cpp:3969-4010`). Rotas: os tipos `objetos` e `controladores` em `GET|PUT|DELETE mapas/{c}/npcgen/{tipo}/{i}`
e `…/clonar` (sem filhos); o resumo `GET npcgen` traz `objetos` `[indice, tipo, x, z, dwDynObjID, idController]`,
`controladores` `[indice, id, iControllerID, nome, bActived, citado_por]` e `controladores_inexistentes_citados`. Regras: o
`id` do controlador não muda (é a chave das citações); clonar dá `id` = maior + 1; remover controlador citado é recusado
(lista quem cita). Painel: objetos como quadrados em "Editar áreas" (com "Objetos" marcado), lista de controladores com
busca, editor com os horários em português e o controlador de cada área/objeto com link.

**Altura do chão (S4, B248):** `GET mapas/{c}/altura?x&z` → `{x, z, y, tem_terreno}`, pelos `.hmap` da pasta do mapa no
realm com o leitor do GS (`pw_data_loader::Terreno`, a mesma conta do `GetHeightAt` do original: `playercmd.cpp:4926`,
`npcgenerator.cpp:4319-4322`; catálogo em `specs/mapas/terreno_155.json`). `y` nulo quando o realm não tem o bloco
daquele ponto (1.2.6: 211 `.hmap` em 42 pastas; 1.5.5: 532 em 82). Terreno em cache por (versão, mapa), até 8 mapas.
Painel: "Posicionar pelo clique" preenche também o Y (área do `npcgen`, renascer, chegada de portal, teleporte); o do
rastreador fica 0, como nos arquivos reais.

**Modelo dos objetos dinâmicos (S4, B250):** `configs\DynamicObjects.data` do cliente publicado, lido por
`DataPathMan::Load` (`EC_Game.cpp:611-614`, `DataPathMan.cpp:9`, `:16-58`): `u32 'PMID'` (`0x504D4944`), `u32 n` e
`n × {u32 id; u32 len; len bytes}` — caminho do `.ecm` em GBK; id ou caminho repetido é recusado, e o arquivo fecha no
último byte (`mapas::ler_objetos_dinamicos`; o do 1.5.5 BR: 235 objetos, fecha; 1.2.6: 553 bytes). O resumo do
`npcgen` ganhou a coluna `modelo` nos objetos e a ficha do objeto o `nome` (o caminho); o painel mostra o nome do arquivo
sem `.ecm`, com o caminho na dica.

**Distritos e portais em par com o cliente (S4, B251):** `pw_data_loader::clt` lê e grava o `precinct.clt` e o
`region.clt` do cliente (texto UTF-16LE com BOM, LF; `CELPrecinct::Load(AWScriptFile*)`, `EL_Precinct.cpp:96-213`;
`CELRegion::Load`/`CELTransportBox::Load`, `EL_Region.cpp:70-100`, `:254-290`) — **ida e volta idêntica nos 104 pares**
dos clientes 1.2.6 e 1.5.5 BR (formato do exportador: inteiros separados por dois espaços, ponto de cidade e vetores do
portal com `", "`, vértices com dois espaços, linha em branco depois de cada registro; `%f` do MSVC, com empate
arredondado para longe do zero: `287.8203125` → `287.820313`). Editar em par (`alterar_sev_e_clt`): o `.sev` vira
item `servidor` e o `.clt` item `cliente` **no mesmo rascunho**, sob a mesma trava, e os carimbos não mudam. Só com o
par **alinhado** (mesmo carimbo; no precinct, mesma contagem e vértices por distrito; no region, a mesma sequência de
regiões e portais — é ela que dá o índice do portal) são permitidos: no distrito, `nome` e `vertices` (`[[x, z]…]`, 3 a
512; a altura vem do 1º vértice antigo — o polígono usa só x e z), `POST distritos {base, campos: {nome, vertices,
ponto?}}` (cópia do base, id no `.clt` = maior + 1, sem as marcas) e `DELETE distritos/{i}`; no portal, `destino`,
`nivel_minimo` (v≥5), `pos`, `exts`, `POST portais {base}` (no fim dos dois) e `DELETE portais/{i}`. Fora disso continua
o que é só do servidor (ponto de renascer, chegada). As respostas `GET distritos`/`portais` trazem
`alinhado_com_o_cliente` e o `motivo`. **Desalinhados hoje:** no 1.2.6, os carimbos de `region` em a27, a31 e nc01 e de
`precinct` em a12, a27 e nc01 diferem entre o cliente e o realm, e a sequência do region em a27 (6 × 1) e a31 (34 × 33);
o 1.5.5 BR casa nos 68 pares. Painel: no distrito, nome, "Redesenhar polígono" (cliques no mapa, barra concluir/
desfazer/cancelar), "Novo distrito (copiar este)" e "Remover"; no portal, destino, nível, caixa e meia-medida,
"Posicionar a caixa pelo clique", "Novo portal" e "Remover"; o corpo do distrito tem limite de 64 KiB no painel.

## 7. Testes

`cargo test -p pw-recursos` — 16 de unidade (inclui a limpeza de objetos) (tar GNU com nome longo e 2º byte 0x5C, GBK, exclusões,
linha de grupo, `sepfile`, assinatura, fluxo do extrator, rotas) e `tests/versao_zero.rs` (com
`TEST_DATABASE_URL`): instalação de mentira → importar → banco `test` → conferir por pasta e por
HTTP com Range (206) → verificar/reparar → atualização da versão 0 para uma 1 montada à mão → manifesto
adulterado recusado; adotar uma pasta sem estado. `tests/governanca.rs`: API com segredo (401 sem),
rascunho idempotente, trava por caminho (409), publicar v1 com mínima, o launcher atualiza uma pasta e
instala outra do zero na v1 (2 por objeto, 2 pulados), reverter → v2 igual à 0. `cargo test -p
pw-atualizador`: tar, nomes. `tests/estudio.rs` (com banco, `elements` e `npcgen` reais do 1.2.6): Estúdio de ponta a
ponta, mapas, e o `npcgen` editado (mover, 2 geradores, clonar, remover), publicado no realm (lido pelos dois
carregadores) e revertido ao original (B243); distritos e portais da a01 (recusas, carimbo intacto, publicar grava
os dois, B244); teleportes do mundo do 1.2.6 com as transportadoras do `elements` e edição publicada (B245).
Rastreador (B246): NPC da a01 sem entrada é acrescentado, `short` estourado recusado, publicado no manifesto (0 aplicados no
realm) e baixado com a posição nova. `pw-data-loader`: `precinct_e_region_de_todos_os_mapas_voltam_identicos`,
`world_targets_volta_identico`, `task_npc::testes`, `campos_de_objeto_e_controlador_nos_npcgen_reais`,
`clt::testes` (os `.clt` reais dos dois clientes, fora do repositório: sem eles o teste avisa). Par (B251): com `.clt`
gerados dos `.sev` da a01, renomear e redesenhar, criar distrito, editar/criar/remover portal → 4 itens no rascunho
(2 `servidor`, 2 `cliente`), publicados, carimbos e sequência iguais, o polígono novo vale no `distrito_em` do GS. Controladores
(B247): 346 na a01, o 456 citado não se remove, `id` não muda, clone com id novo, publicado e lido pelo GS. Altura (B248):
sem `.hmap` responde `y` nulo; coordenada inválida 400. `pw-data-loader`: `npcgen_de_todos_os_mapas_volta_identico` (117
arquivos). Painel: `web-admin/backend/tests/teste_recursos.py` (10) — rodar no contêiner da imagem do painel com
`TEST_DATABASE_URL`/`TEST_REDIS_URL` (ver histórico B243).

## 8. Falta

- Launcher: canal de teste para GM (instalar um rascunho antes de publicar), autoatualização do próprio
  launcher, instalador `.msi` (CLI do Tauri).
- ("Aguarda reinício" e reiniciar pelo painel: feitos no B249, spec 06.) Rascunho só de servidor continua gerando
  versão nova do cliente com o manifesto igual — **decisão do B250, mantido**: o launcher só baixa o manifesto e termina,
  e a versão é o que guarda o `publicada_em` do aviso de reinício e o que o reverter usa.
- Estúdio: telas próprias, `tasks`/`gshop`; no mapa, destinos de teleporte novos (exigem o `world_targets.txt` do
  cliente), regiões (`[region]`, a zona segura) editáveis, nome de controlador (o `szName` é só leitura); alinhar os
  carimbos dos 4 mapas do 1.2.6 que divergem.
- Envio do cliente original pelo painel para importar no servidor externo (pasta enviada arquivo a
  arquivo, retomável — decisão B233).
- Pacotes novos de tempos em tempos (hoje a versão N reaproveita os da 0 e o resto vai por objeto).
- Troca de chave (`chaves.json` assinado pela anterior, §7.2).
