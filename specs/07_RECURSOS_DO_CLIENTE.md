# 07 — Recursos do cliente (`pw-recursos`)

> Como o servidor guarda o cliente de cada versão e o entrega ao launcher. Plano e decisões:
> `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (§3, §7.2, §7.7, §8, §8.6). Estado: **P1
> (versão 0 + pacotes + HTTP) no B233, confirmado em jogo nas duas versões; P2 (núcleo do launcher
> `pw-atualizador` + app `tools/pw-launcher`, Tauri no B234 e Win32 nativo desde o B275) no B234, confirmado em jogo; usabilidade do launcher e P3
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
| app | `tools/pw-launcher` (Win32 puro via `windows-sys` 0.52, fora do workspace; **compilar com `compilar.ps1`** na pasta — alvo `x86_64-win7-windows-msvc`, ou `-Bits 32` para `i686-win7-windows-msvc`) | janela `src/janela.rs` (era Tauri/WebView2 até o B275, que exigia Windows 10); roda do **Windows 7 ao 11**; `pw-launcher.json` ao lado do `.exe` guarda o servidor, a última versão escolhida e **uma pasta por versão** (`pastas`); chave pública embutida de `chave_publica.txt` (trocar ao publicar com outra chave) |

Tela (B235; Win32 no B275, versões numa caixa de seleção em vez de abas): ao abrir, busca as versões no servidor e mostra a situação da escolhida, lendo só o
`versao.json` (`Servidor::versao_anunciada`, sem baixar o manifesto). Conforme a situação:

| situação | botões |
| :--- | :--- |
| sem pasta / pasta sumiu / pasta sem o jogo | **Baixar o jogo** (janela de pastas do Windows; instala na escolhida se estiver vazia, senão em `PW Universal <versao>` dentro dela) e **Já tenho instalado** (escolhe a pasta `element`, ou a pasta acima dela; adota: grava manifesto e estado e repara o que divergir — `cliente::adotar`) |
| download interrompido | **Continuar download** (retoma) e escolher outra pasta |
| instalado | **Jogar** (ou **Atualizar** + "Jogar assim mesmo" quando o servidor tem versão nova), **Verificar arquivos**, **Abrir pasta** (Explorer), **Trocar pasta**; no alto, **Opções do jogo…** |

O endereço do servidor fica no rodapé ("Servidor de atualização" + "Salvar e reconectar"); sem esquema
vira `http://` (`192.168.1.13:8100` → `http://192.168.1.13:8100`).

**Endereço do jogo pelo servidor de atualização (B280).** Os serviços do pw-universal rodam na mesma
máquina; por isso o host do servidor de atualização é também o do jogo. Na `serverlist.txt` publicada os
servidores nossos apontam para `127.0.0.1`, que quer dizer "a máquina do servidor de atualização". O
launcher (`tools/pw-launcher/src/local.rs`) troca, em `userdata\server\serverlist.txt` (UTF-16LE com BOM;
linha `nome<TAB>portas:endereço[:linha][<TAB>zona]`, `EC_ServerList.cpp:75-160`) e no
`CurrentServerAddress=porta:ip` do `userdata\currentserver.ini` (`EC_ServerList.cpp:497-536`), o endereço
que for `127.0.0.1`/`localhost` ou um host que ele mesmo pôs antes (`hosts_jogo` no `pw-launcher.json`)
pelo host atual. Linhas com outro endereço (servidores de terceiros) ficam. Quando: depois de
instalar/atualizar/adotar/reparar, antes de **Jogar** e ao salvar o servidor (em todas as versões com
pasta). Mudou o IP: trocar o servidor de atualização no launcher basta.

**Lista publicada (B281, versão 1 das duas linhas):** só o realm que existe, com o nome da tabela `realms`:
1.5.5 = `pw-universal` (grupo, obrigatório no 1.5.5) + `Perfect World Evolved (1.5.5)<TAB>29004:127.0.0.1<TAB>2`;
1.2.6 = `Perfect World Classic (1.2.6)<TAB>29000:127.0.0.1`. Saíram `Universal 1.5.5` (29003, realm EN apagado em
2026-09-17) e os servidores de teste `BATATA`/`Batata_E` do 1.2.6. Realm novo ou renomeado: publicar a lista de novo
(rascunho com `userdata\server\serverlist.txt`, destino `cliente`).

**Opções do jogo (B280).** Janela com modo (janela/tela cheia), resolução (as da placa acima de 400x400,
`DlgSettingSystem.cpp:290-293`, mais a atual), widescreen (só a partir de 800x600,
`DlgSettingSystem.cpp:483-494`), qualidade 0–4 e volumes de som e música 0–100 (faixas do
`setting_system.dtf:33,42-43`). Grava em `userdata\systemsettings.ini` as chaves que o cliente lê
(`EC_Configs.cpp:403-431`; padrões quando faltam: 800x600, tela cheia, sem widescreen, `Level` 0, volumes
50): `RenderWid`, `RenderHei`, `FullScreen`, `WideScreen`, `SoundVol`, `MusicVol` e, com o `Level`, as
chaves que o diálogo do cliente muda junto (`DlgSettingSystem.cpp:326-440`: `Sight`, `WaterEffect`,
`SimpleTerrain`, `Tree/Grass/CloudDetail`, `Shadow`, `MipMapBias`, `FullGlow`, `SpaceWarp`, `SunFlare`,
`AdvancedWaterUD`, supondo a placa capaz). O resto do arquivo é preservado. O 1.2.6 grava as mesmas chaves
(arquivo dele conferido; sem `ScaleUI`).

**Arquivos do jogador (B280, `pw_atualizador::cliente::do_jogador`).** Tudo sob `userdata\` é do
jogador: o próprio cliente reescreve opções, servidor escolhido e layout por personagem, e o launcher
ajusta a `serverlist.txt`. **Verificar/Reparar** só repõem o que faltar (conteúdo diferente não é
estrago). **Atualizar** troca um arquivo do jogador só se ele ainda for o da versão antiga (e só apaga
nessa condição); a `serverlist.txt` é exceção (`segue_o_servidor`): segue sempre o servidor, e o launcher
reaplica o endereço depois.

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

**Destinos de teleporte em par com o cliente (S4, B252):** `world_targets::TxtDosDestinos` lê e grava o
`maps\world\world_targets.txt` (UTF-16LE com BOM, CRLF, `id ⇥ "nome" ⇥ mundo ⇥ "x,y,z" ⇥ domínio`, posição em `%.3f`
do MSVC — `clt::formatar_com`) — **idêntico nos clientes 1.2.6 (49) e 1.5.5 BR (92)**, e nos dois o `.txt` e o `.sev`
têm os mesmos ids na mesma ordem e as mesmas posições (o `.sev` sai do `.txt` pelo `globalsave`). Edição em par
(`alterar_destinos`): `world_targets.sev` item `servidor`, `.txt` item `cliente`; com os ids alinhados a posição vai aos
dois **arredondada a 3 casas**, como o `globalsave` faria (sem o `.txt`, só o `.sev`, exata). Só alinhado: `nome`,
`POST mapas/{c}/teleportes {base, campos: {nome, pos}}` (cópia do domínio do `base`, no mapa aberto, id = maior + 1) e
`DELETE teleportes/{id}` — **recusado se alguma `NPC_TRANSMIT_SERVICE`** (no `elements` do rascunho) ainda leva ao
destino. Destino novo fica sem transportadora até a ficha do serviço listar o id (`targets_N_idTarget`,
`num_targets`). Painel: nome, "Novo destino (copiar este)", "Remover destino" (a rota genérica `POST …/{arquivo}` do
painel aceita `teleportes`).

**Alinhar o servidor com o cliente (S4, B253):** `PrecinctDoCliente::para_o_servidor`/`RegionDoCliente::para_o_servidor`
geram o `.sev` a partir do `.clt` (os campos são os mesmos; a versão é a do `.sev` do realm, o carimbo o do cliente).
O `.clt` guarda `%f` (6 casas), que nem sempre devolve o `float` exato que o `.sev` tem (o editor exporta os dois da
memória), então a comparação é por **equivalência** (`clt::precinct_equivalente`/`region_equivalente`: carimbo,
contagem e inteiros iguais, `float` a < 1e-3). Medido nos 200 pares dos dois clientes: **186 equivalentes, 14
diferentes** — 1.2.6 `precinct` a12, a25, a27 e `region` a25, a27, a31; 1.5.5 BR `precinct` a25, a32, a40, a43,
**a61** (o mapa onde todo personagem novo nasce: 27 distritos no cliente, 25 no realm), a70, b01, random03 — nesses do
1.5.5 o carimbo é igual (o cliente não reclama), mas renascer usa os distritos do servidor. Rotas:
`GET estudio/alinhamento[?rascunho]` (todos os mapas do `instance.txt`: por arquivo `equivalente`, carimbos e contagens) e
`POST mapas/{c}/alinhar?rascunho` (refaz no rascunho, como item `servidor`, só os `.sev` que não são equivalentes; nada
vai ao cliente). Painel: "Alinhar o servidor com o cliente" no editor de distrito/portal quando o par não casa. **Não
aplicado a nenhum mapa** — qual lado vale (cliente BR ou pacote de servidor) é decisão do Murillo. Regiões (`[region]`,
zona segura) seguem só leitura: no servidor elas só servem aos pontos de passagem (`ActivateRegionWayPoints`,
`player.cpp:25193-25215`), efeito em jogo de editá-las ainda não medido.

**Loja Gold (S7, B254):** `data\gshop.data` (e `gshop1`/`gshop2` no 1.5.5) — o realm usa **o mesmo arquivo** do cliente
(o índice da oferta é o `goods_index` do `MALL_SHOPPING`, spec 03 §3.8; o SHA do publicado é igual ao do `config` do
realm nas duas versões), então a edição é um item **`ambos`** (`gravar_item_com_destino`), como o `elements`: publicar
põe no manifesto e grava `config/gshop*.data`; o carimbo não muda. Rotas: `GET estudio/loja/{arquivo}` (8 categorias com
subcategorias, linhas `[indice, main_type, sub_type, id, num, szName, buy_0_price, item]` com o nome do item no
`elements`, e os campos do layout), `GET|PUT|DELETE estudio/loja/{arquivo}/{i}` (`PUT {campos}` pelos nomes de
`gshop_editavel`) e `POST estudio/loja/{arquivo} {base}` (cópia no fim — ninguém muda de índice; remover sobe os
seguintes, nos dois lados juntos). Painel: vista "Loja Gold" do Estúdio (`static/loja.js`): arquivo, categoria, busca,
editor por grupos (identificação, 4 compras, descrição, brinde/donos/limite) com link para a ficha do item. Unidade do
preço: a do arquivo, sem conversão. **Categorias (B257):** `POST estudio/loja/{arquivo}/categorias {acao, categoria,
sub?, nome?}` — `renomear` (categoria ou subcategoria, `wchar[64]`: até 63 caracteres; mesmo nome não muda nada),
`nova` (subcategoria no fim: nenhuma oferta muda de índice) e `remover` (só a **última** subcategoria e só se nenhuma
oferta a usa — `main_type`/`sub_type` são índices). Painel: botão "Categorias" na vista Loja.

**Comportamento dos monstros (S7, B258 — só leitura):** o `aipolicy.data` do realm (só do servidor; o cliente não o
tem) pelo leitor do GS (`pw_data_loader::aipolicy`, porte de `cgame/gs/ai/policy.cpp`, interpretado no mundo desde o
B127), em cache por versão/SHA. `GET estudio/comportamentos?busca` (linhas `[id, gatilhos, monstros]`, busca por id
da política ou id/nome de monstro; até 500; `sem_politica` = ids citados por monstros que o arquivo não tem) e
`GET estudio/comportamentos/{id}` (a `AiPolicy` serializada — gatilhos com `ativo`, `rodando`, `so_em_combate`,
condição em árvore e operações com alvo — e os monstros que a usam por `MONSTER_ESSENCE.common_strategy`,
`monstros.rs:169`). Painel: vista "Comportamento" (`static/comportamento.js`). **Editar no lugar (B259):** item
**`servidor`** (`servidor\aipolicy.data`; o cliente não o tem). A ficha traz `editaveis` por gatilho (`condicoes`
por caminho na árvore — `""`, `"e"`, `"d"`… — e `operacoes` por índice, cada um com `[[campo, tipo]]`) e
`PUT estudio/comportamentos/{id}?rascunho {gatilho, flags?, operacao?: {indice, valores}, condicao?: {caminho,
valores}}` grava (campo inexistente → 400; relê antes). A leitura usa o rascunho quando há um. Painel: "Editar este
gatilho" com as flags e os campos de cada operação/condição editável. **Estrutura (B261):** o mesmo `PUT` aceita
`fala: {indice, texto}` (operações `Fala`, `Fala2`, `SkillComFala`), e por último `duplicar: {novo_id?}` (resposta
`resultado.novo_gatilho`) ou `remover: true` — ver spec 03 §aipolicy. Painel: "Salvar fala" por fala, "Duplicar
gatilho" e "Remover gatilho" (com confirmação). **Tipos (B271):** o `PUT` aceita ainda, aplicados antes das edições de
valor, `tipo_da_condicao: {caminho, tipo}`, `tipo_da_operacao: {indice, tipo, id_gatilho?}`, `operacao_nova: {tipo,
id_gatilho?}` e `remover_operacao: indice`; a ficha traz `tipos` (`condicoes: [[tipo, nome, é_valor]]`,
`operacoes: [[tipo, nome]]`, só os que o arquivo usa). Painel: seletor de tipo por operação e por folha de condição,
"+ Operação" e "Remover".

**Textos do cliente (B260, S7):** `pw-recursos/src/textos.rs` lê e grava `configs\fixed_msg.txt`, `item_desc.txt` e
`item_ext_desc.txt` — o cliente os carrega em `CECGame::Init` (`EC_Game.cpp:517-525`) por
`CECStringTab::LoadWideStrings` (`EC_StringTab.cpp`): UTF-16LE com BOM; cabeçalho até `#_begin`; `#_index` antes dele
torna a tabela **indexada** (`id "texto"`; id repetido vale o primeiro) — senão **sequencial** (`"texto"`, a chave é a
posição). Edição por linha: só a linha mudada é reescrita, o prefixo até a primeira aspa fica; texto com aspas ou
quebra de linha → 400 (o leitor não tem escape); entrada nova só na indexada, antes das linhas vazias do fim. Os três
arquivos do 1.5.5 BR voltam idênticos. Medido nos realms publicados: só `item_ext_desc` é indexada (1.2.6: 3.620
entradas; 1.5.5: 16.941); `item_desc` (202; 412) e `fixed_msg` (173; 287) são sequenciais. Item **`cliente`** (`configs\<arquivo>`, não vai ao realm).
`GET estudio/textos/{arquivo}?busca&pagina&por_pagina&rascunho` → `{indexada, entradas, total, linhas: [[chave,
texto]]}` (busca por chave exata ou trecho do texto; 100 por página, até 500); `PUT estudio/textos/{arquivo}/{chave}
{texto}` e `POST estudio/textos/{arquivo} {id, texto}` com `?rascunho`. Painel: vista "Textos" (`static/textos.js`).

**Missões (S7, B255 — só leitura):** o `tasks.data` do realm (o que o GS executa, `manager.rs:461`; 20,8 MB no 1.2.6,
104 MB no 1.5.5) pelo leitor do GS (`pw_data_loader::tasks`), em cache por versão e SHA. Rotas:
`GET estudio/missoes?busca&topo&pagina&por_pagina` (linhas `[id, nome, tipo, nível mín., máx., mãe, filhas, npc que
entrega, npc que premia]`; busca por id ou nome; `topo` = só as de topo) e `GET estudio/missoes/{id}` (o
`TaskTemplate` inteiro serializado, mais `nomes` — todo id citado em campo de id/NPC/item/monstro/brinde com o nome no
`elements` publicado — e mãe, filhas, pré-missões, exclusivas e a missão dada no prêmio com nome). Painel: vista
"Missões" (`static/missoes.js`), lista paginada, ficha por grupos (quem dá/premia, requisitos, objetivo, ao receber,
prêmio, falha) e "Demais campos". **Editar no lugar (B256):** o `tasks.data` do cliente publicado é o mesmo do realm
(SHA igual nas duas versões), então a edição é um item **`ambos`** (`data\tasks.data`); a leitura passa a ser a do
cliente publicado, ou do rascunho quando há um. `GET estudio/missoes/{id}` traz `editaveis` (`[nome, tipo]` da
`tasks_editavel::campos`) e `PUT estudio/missoes/{id}?rascunho {campos}` grava (relê pelo leitor do GS antes; campo de
tamanho variável → 400). Painel: formulário "Editar" na ficha da missão, com o aviso de que mudar o NPC que entrega
não muda o `NPC_TASK_OUT_SERVICE` do `elements`. **Textos (B262):** a ficha traz `textos` (`descricao`, `ok`, `nao`,
`tributo`, decifrados) e o mesmo `PUT` aceita essas chaves em `campos` (gravadas depois dos fixos; mudam o tamanho e
a tabela de deslocamentos — spec 03). Painel: uma caixa de texto por texto longo no formulário da missão.
**Listas (B263):** a ficha traz `listas` (`[nome, capacidade]`) e `campos` aceita `pre_tasks`, `missoes_exclusivas`
e `req_classes` como lista de inteiros (no lugar; regras na spec 03). Painel: ids separados por vírgula.
**Vetores (B264):** a ficha traz `vetores` (`[nome, teto]`) e `campos` aceita `itens_exigidos`, `itens_entregues` e
`monster_kills` como a lista inteira de registros (o JSON da ficha). Painel: uma linha por registro com os campos
principais, "+ Linha" e "Remover"; os outros campos do registro são preservados. **Prêmio em itens (B265):** a ficha
traz `grupos_do_premio` (tetos) e `campos` aceita `premio_grupos` e `falha_grupos` (o JSON de `grupos_de_itens`).
Painel: editor de grupos ("sorteia um", itens com id, quantidade e peso). **Diálogos (B266):** a ficha traz
`dialogos` (os cinco, com `nome`, `id_talk`, `texto`, `janelas[{id, pai, texto, opcoes[{id, texto, param}]}]`) e
`campos.dialogos = [{dialogo, janela?, opcao?, texto}]` troca título, janela ou opção. Painel: um bloco recolhível por
diálogo, com título, texto de cada janela e de cada opção. **Estrutura (B270):** na mesma lista, `acao: "nova_opcao"`
(`janela`, `texto`, `destino: {janela: texto} | {funcao: n}`, `param?`), `"remover_opcao"` (`janela`, `opcao`) e
`"criar_raiz"` (`texto`); a ficha traz `funcoes_de_opcao`. Painel: "+ Opção" (abre janela nova ou função), "Remover"
por opção e "Criar janela inicial" em diálogo vazio. **Clonar (B267):** `POST
estudio/missoes/{id}/clonar?rascunho {novo?}` (padrão: maior id + 1) → `resultado.id`; missão de topo, com as
submissões (B269: ids consecutivos a partir de `novo`). A missão nova não é oferecida em jogo até entrar no `NPC_TASK_OUT_SERVICE` de um NPC (Tabelas). Painel:
"Clonar missão" na ficha (proxy recusa id fora de 1..65535). **Remover (B268):** `DELETE
estudio/missoes/{id}?rascunho` tira a missão de topo e a árvore dela (`resultado.missoes_removidas`); 400 com a lista se
o `elements` do rascunho cita a missão ou uma submissão — campos `int32` com "task" no nome, menos serviço, quantidade,
probabilidade, item, custo e contagem (no v156: `id_tasks_N`, `tasks_N_id_task`, `task_lists_N_id`, `list_N_task_id`,
`task_N`; larga de propósito) — ou se outra missão a cita. Painel: "Remover missão" (com confirmação).

**Modelos 3D (B272, S5):** `GET estudio/modelos?busca` (os `.ski` da versão publicada cujo caminho contém a busca;
`{total, linhas}`, até 200), `GET estudio/modelos/ficha?caminho` (versão, ossos, texturas, materiais, malhas com
vértices/triângulos) e `GET estudio/modelos/glb?caminho` (`model/gltf-binary`, spec 03 §3.10d). Painel: vista
"Modelos" (`static/modelos.js`) com busca, ficha, visualizador **WebGL próprio** (lê o GLB; sem biblioteca externa, a
CSP `self` continua) — arrastar gira, roda aproxima — e "Baixar GLB" para o Blender. **Texturas (B273):** a ficha
traz `texturas_achadas` (o caminho onde o cliente acharia cada textura, ou nulo) e o GLB leva as texturas em PNG
(spec 03 §3.10d); o visualizador as desenha, com corte de alfa nas transparentes, e a ficha lista as não achadas.
**Modelo inteiro (B274):** a lista traz `.smd` (primeiro) e `.ski`; para um `.smd` a ficha traz `tipo: "modelo"`, as
ações, o esqueleto (`ossos [[nome, mãe]]`, juntas, ganchos, FPS, ou `erro` — p.ex. `.bon` v5) e cada pele (malhas e
triângulos, ou `erro`), e o GLB junta as peles (com as texturas de cada uma) e o esqueleto (`skin`, spec 03 §3.10d).
**Ações (B276):** a ficha do `.smd` traz `lista_de_acoes` (`nome`, `inicio_ms`, `fim_ms`, `stck`) e
`acoes_descartadas`; `GET estudio/modelos/glb?caminho&acao=<nome>` traz a animação dessa ação (trilhas do `.bon` v5 ou
o `.stck`; ação inexistente ou descartada → 400). Painel: seletor de ação na ficha; o visualizador toca a animação em
laço, com a pele deformada na CPU a cada quadro. **Diagnóstico (B277):** a lista traz também os `.ecm` (primeiro); a
ficha do `.ecm` traz o `.smd`, as ações combinadas (com as ações-base ausentes no `.smd`) e `diagnostico_de_monstro`:
para cada nome pedido (tabela `configs\actions_npc.txt` **do próprio cliente da versão** + `起`/`落`), `ok`, "sem ação
combinada no .ecm", "só existe como ação-base no .smd" (o motor 1.5.5 só toca combinadas, `EC_Model.h:577-583`),
"ação combinada sem ação-base (não anima)" ou "ação-base ausente no .smd" (spec 03 §3.10d). Medido em 40 monstros por
realm: no 1.2.6 a única falta é `受伤叠加`, presente só como ação-base (o caso que o comentário de
`EC_NPCModel.cpp:693-697` descreve); no 1.5.5, 38/40 sem ferido e um modelo (`白虎boss`) sem guarda, movimento e
ataques combinados.
Painel: tabela do diagnóstico e o link para abrir o `.smd` e tocar as ações. **Modelo sem pele lida (B278):** o
`.smd` pode não citar pele (corpo de jogador — a pele vem do equipamento) ou citar uma que falta no próprio cliente
(p.ex. `players\通用装备\飞剑\飞剑10\飞剑.smd` cita `飞剑.ski`, que não está na pasta; o modelo em uso é o
`飞剑10.ecm → 飞剑10.smd`). Com esqueleto, o GLB sai só com ele (sem `meshes`, que o glTF não aceita vazio) e o
visualizador desenha os ossos em linhas, animados; sem esqueleto, 400 com as peles citadas. A lista traz cada tipo com
a sua cota (`por_tipo`: `ecm`, `smd`, `ski`; até `limite` de cada). **Da ficha ao modelo (B279):** na ficha de
qualquer tabela, um campo de texto cujo valor termina em `.ecm`/`.smd`/`.ski` (`file_model`, `file_model_right`…) ganha
o link "ver o modelo" (no `.ecm`, "e o diagnóstico de ações"), que abre a vista Modelos nele. Medido: em 40 monstros
por realm, os 34 com `file_model` preenchido abrem a ficha do `.ecm` (os outros 6 têm o campo vazio).
**Corrigir ação (B280):** `POST estudio/modelos/acao?rascunho {caminho, origem, nova, base?}` cria a ação combinada
`nova` no `.ecm` clonando `origem` (item `cliente`; regras na spec 03 §3.10d); a ficha do `.ecm` aceita `rascunho` (lê
o do rascunho) e traz `bases_do_smd`. Painel: em cada pedido do diagnóstico que não está ok, "Criar" com a ação a
copiar e a ação-base. **Mascote (B281):** a ficha do `.ecm` aceita `habitat` (0–6) e traz `diagnostico_de_mascote`
(os nomes com `_地面`/`_水中`/`_空中`, spec 03 §3.10d). Painel: seletor "diagnóstico como monstro / como mascote
(habitat)"; da ficha de um registro com `inhabit_type` e `id_type` 28752 ou `inhabit_type` 6, o link do modelo já abre
como mascote no habitat dele. **Modelo a partir do `.ecm` (B282):** `GET estudio/modelos/glb?caminho=<.ecm>&acao?` monta
o modelo como o jogo: o `.smd` do `.ecm` (esqueleto e ações) com a pele `<.ecm>.ski`; sem ela, as `AddiSkinPath`
(`Models\` + caminho) e por fim as peles do `.smd` (spec 03 §3.10d). A ficha do `.ecm` mostra esse modelo (malha,
textura, esqueleto) com o seletor das ações do `.smd` e o "Baixar GLB"; o `.smd` sozinho continua acessível ("Abrir só o
.smd"). **Diagnóstico de voo (B283):** a ficha do `.ecm` aceita `voo` (0 espada, 1 asa, 2 rodas) e traz
`diagnostico_de_voo`, com estes campos:
- `encaixe` (`CC_feijian`/`CC_chibang`) e `encaixe_presente` nos ganchos do esqueleto do `.smd`;
- `jogadores`: os `.ecm` de `Models\Players\形象\` lidos;
- `colegas`: os outros modelos de voo do mesmo modo no elements (`Estudio::modelos_de_voo`: `file_model`/`file_model2`
  de `FLYSWORD_ESSENCE` com `fly_mode`, e de `WINGMANWING_ESSENCE` como asa);
- `acoes`: por nome mandado, quantos jogadores o mandam, quantos colegas o têm e o estado (spec 03 §3.10d). A falta
  só é acusada se a maioria dos colegas tem a ação; senão, o estado é "ausente, como na maioria dos colegas" (sem ⚠).

O caminho do `string` do layout vem em bytes GBK (`FieldValue::Raw`), convertido com `nomes::legivel`, como na ficha.
O painel repassa `voo` (0–2) na ficha. Até o B284 ele não repassava, e o perfil de voo pelo painel caía no diagnóstico
de monstro; agora `teste_recursos.py` cobre o repasse.

**Onde nasce (B284):** `GET estudio/nascimentos?id=N&rascunho?` (`Estudio::onde_nasce`) percorre os mapas do
`configs\instance.txt` que têm npcgen e lista cada área que cita o id:
- de criatura: geradores com id em `[0..4]` e quantidade em `[4..8]`; tipo `monstro`/`npc` pelo `iNPCType`;
- de recurso: id em `[4..8]` e quantidade em `[12..16]`.

Cada linha traz `{mapa, caminho, tipo, area, x, z, quantidade}`, e o resultado traz também `mapas_lidos`. Com rascunho,
vale o npcgen do rascunho. Medido: 0,2 a 0,3 s no 1.2.6 (42 mapas) e até 1,4 s no 1.5.5 (80 mapas).

No painel, as fichas de `MONSTER_ESSENCE`/`NPC_ESSENCE`/`MINE_ESSENCE` ganham "Onde nasce", que só busca ao ser
aberto. Clicar num local abre a vista Mapas nesse mapa, já com a área aberta (`irParaArea`, `mapa.js`).

**Minas não têm diagnóstico de ações.** Toda matéria com modelo de pele pede só `掉落保护`, ou `_水中`/`_空中` conforme a
altura (`CECMatter::SetLoadedMatterModel`, `EC_Matter.cpp:298-312`); a mina carrega como `CECModel` comum, e sem
modelo usa `models\error\error.ecm` (`:71-95`). Medido: nenhum dos modelos de mina amostrados (60 por versão) tem essa
ação, e só 4 têm alguma (`站立`/`四处张望`). A mina original fica parada de propósito, então acusar falta seria ruído.

**Copiar ação de outro modelo (B285):** `GET estudio/modelos/doadores?caminho=<.ecm>&acao=` (`Estudio::doadores`) lista
até 50 doadores, com o `total` e quantos estão `prontos`. Doador é outro `.ecm` da mesma `Version` que tem a ação
combinada. Cada um traz `mesmo_smd`, `mesmo_esqueleto` e `bases_faltando` (as ações-base dele que o `.smd` do destino não
tem); "pronto" é não faltar nenhuma. O esqueleto não decide: a combinada cita as ações-base pelo nome, e o motor as
procura no `.smd` de quem toca. Ele só ordena, porque os efeitos citam ganchos (`HookName:`) que outro esqueleto pode
não ter. A ordem é: prontos primeiro, depois mesmo `.smd`, mesmo esqueleto e menos faltas. A 1ª versão exigia o mesmo
esqueleto, e assim os 118 pedidos de uma amostra ficaram sem doador: os irmãos de esqueleto são cópias e não têm a ação
que falta.

A busca usa `Estudio::indice_de_ecm`: todos os `.ecm` e os `.smd` que eles citam (ações, esqueleto e ações-base),
guardados em memória por versão e montados uma vez por `manifesto_sha256`.

`POST estudio/modelos/acao` aceita `de` (o `.ecm` de origem, do cliente publicado; não combina com `base`). Ele recusa
com 400 se alguma ação-base da ação copiada não existe no `.smd` do destino — sem ela, a ação não animaria.

No painel, cada falta do diagnóstico ganha "Copiar de outro modelo…". Os doadores com base faltando aparecem
desabilitados. Teste: `doador.ecm` sintético do mesmo `.smd`; `休闲` copia, e `普攻2`, com base ausente, dá 400.

**Importar glTF como pele (B290, S6):** `POST estudio/modelos/importar?rascunho=&smd=&destino=`, com o GLB no corpo
(`Estudio::importar_pele`).
- O esqueleto é o `.bon` citado pelo `.smd`, lido com o rascunho.
- A pele vem de `importar::ski_do_glb` (spec 03 §3.10d) e é gravada no `.ski` de destino do rascunho, como item
  `cliente`. Por enquanto só substitui um `.ski` que já existe no cliente publicado.
- A resposta traz `resultado: {malhas, vertices, ossos, texturas, esqueleto, relatorio}`.
- Sem rascunho, com corpo que não é GLB, ou com osso que não existe no esqueleto, a resposta é 400.

No painel, cada pele da ficha do `.smd` ganha "Substituir por GLB…", e o relatório aparece no aviso. Essa rota aceita
corpo de até 64 MB (`seguranca.eh_importacao_de_modelo`); nas outras o limite continua 8 KiB. Medido pelo
`pw-recursos`: o GLB de 10 `.smd` de NPC por versão, importado de volta como a 1ª pele, deu 10 de 10 nas duas versões,
sem nada a relatar, em até 1,4 s.

**Pele nova (B291, S6):** `POST estudio/modelos/pele_nova?rascunho=&smd=&nome=`, com o GLB no corpo
(`Estudio::nova_pele`), cria a pele em `<pasta do .smd>\<nome>` (o motor monta `strPath + "\\" + strFile`,
`A3DSkinModelMan.cpp:163-170`) e põe o nome no fim da lista de peles do `.smd` (`smd::Smd`). Os dois ficam no
rascunho: o `.ski` como item `adicionar` e o `.smd` como `substituir`.
- **Recusas (400):** nome com pasta ou sem `.ski`, fora do GBK, já existente no cliente (para isso há "Substituir"),
  já citado pelo `.smd`, ou `.smd` sem esqueleto.
- **Ordem:** o `.smd` novo é montado e o GLB importado antes de qualquer gravação.
- **Atenção:** NPC e monstro não usam as peles do `.smd`; o jogo põe `<.ecm>.ski` (B282).
- **Painel:** ao fim da lista de peles da ficha do `.smd`, "Pele nova a partir de GLB…" com o campo do nome (sem
  nome, usa o do arquivo).

**Texturas da importação (B292):** "Substituir" e "Pele nova" gravam no rascunho cada textura que o GLB traz em
`<pasta da pele>\Textures\<nome>`, o primeiro lugar onde o cliente procura (B273). Se o cliente já tem o arquivo com
os mesmos pixels onde o cliente o acha hoje (em `Textures\` e, sem ela, em `Tex_<título da pele>\`), ele não é
gravado; senão, uma cópia em `Textures\` passaria a valer no lugar da original. Medido pelo `pw-recursos` em 5
`.smd` de NPC por versão, em rascunhos descartados:
- ida e volta (o GLB baixado do painel): 13 de 13 texturas "igual";
- com a imagem renomeada: 10 de 10 "adicionada" em `…\Textures\<nome>.dds`.

B293: a textura gravada é DXT1/DXT5 (spec 03), e a comparação usa os pixels de origem (o PNG), não o DXT, que tem
perda.

**Ações de GLB (B295, S6):** `POST estudio/modelos/acoes_glb?rascunho=&smd=`, com o GLB no corpo
(`Estudio::importar_acoes`), transforma cada animação numa ação do `.smd`.
- **Arquivos:** a ação vira `<pasta das trilhas>\<ação>.stck` (sem pasta, `tcks_<título>`) e uma entrada
  `FILEACTIONDATA6` (quadros 0 a n−1) mais o nome da `.stck` no `.smd`. É a convenção do cliente: 249 de 249 ações
  amostradas com `<ação>.stck` e quadro inicial 0.
- **Nome repetido:** a ação de mesmo nome é substituída.
- **Escopo:** `.smd` v ≥ 7 com `.bon` v6 grava uma `.stck`. B296: com `.bon` v5 (todo o 1.2.6), a ação é emendada na
  trilha embutida (`importar::anexar_acao_v5`), e o rascunho ganha o `.bon` e o `.smd`; a resposta traz o `intervalo`
  de quadros. `.smd` < 7 com `.bon` v6 dá 400, porque o motor descarta as ações dele.
- **Resposta:** `acoes: [{nome, stck, quadros, trilhas, fps, acao}]`, `relatorio` e `acoes_do_modelo`.
- **Para o jogo tocar:** a ação ainda precisa de uma ação combinada no `.ecm`, feita pelo "Criar" do diagnóstico
  (B280) com essa base.
- **Painel:** "Importar ações de GLB…" na ficha do `.smd`.
- **Igual ao cliente:** se a `.stck` gerada é igual à do cliente, ela não é regravada e a ação sai "igual à do cliente
  (não gravada)". Se nada muda no `.smd`, não há item.

**Modelo novo a partir de GLB (B298, S6):** `POST estudio/modelos/novo?rascunho=&pasta=&titulo=`, com o GLB no corpo
(`Estudio::modelo_novo`), cria em `<pasta>\<título>.*` e no rascunho:
- o `.bon` (`esqueleto_do_glb`);
- o `.ski` (`ski_do_glb`), com as texturas em `Textures\`;
- o `.smd` v8;
- as ações em `tcks_<título>\<ação>.stck`.

A convenção segue os 2.980 `.smd` v8 do cliente 1.5.5 (medida):

| item | valor | quanto do cliente |
| :--- | :--- | ---: |
| marca e reservado | `MOXB`, reservado zero | 100% |
| esqueleto | `<título>.bon` | 98,6% |
| 1ª pele | `<título>.ski` | 93% |
| física | vazia | 61% |
| pasta das trilhas | `tcks_<título>`; vazia quando não há ações | 59% |

Tudo é montado e conferido antes de gravar. É recusada (400) a pasta fora de `models\`, o título com pasta ou ponto,
o nome fora do GBK e o arquivo que já existe no cliente. Painel: abaixo da busca da vista Modelos, os campos pasta e
título e o botão "Modelo novo a partir de GLB…".

B299: o modelo novo também ganha `<título>.ecm`, que é o que a tabela (`file_model`) cita para NPCs e monstros. O jogo
põe a pele `<.ecm>.ski` (B282), que é a `<título>.ski` ao lado. O `.ecm` vem de `ecm::ecm_novo`, com uma ação combinada
por ação de mesmo nome. O molde é o parâmetro `molde=` ou, sem ele, entre os `.ecm` da versão de `.ecm` mais comum da
versão de jogo, o 1º em ordem de caminho que serve (reproduzível). A resposta traz `ecm` e `molde_do_ecm`. O `.ecm` novo **herda o cabeçalho do molde** (as configurações gerais do modelo). O automático
saiu `投石车状态.ecm` (uma catapulta) no 1.2.6 e `灯笼.ecm` (uma lanterna) no 1.5.5; não há evidência de que um seja
melhor que outro para um monstro. Por isso o painel tem o campo "molde .ecm" (ex.: um monstro parecido). Medido pelo
`pw-recursos`, em rascunhos descartados: 5 NPCs por versão viraram modelo novo com o `.ecm`, 5 arquivos cada.

**Extensão do Blender (B300, S6):** `tools/blender/pw_estudio`, no formato de extensão do Blender 4.2+
(`blender_manifest.toml`). Foi testada no Blender 5.1.2. Para empacotar e instalar:

```
blender --command extension build --source-dir tools/blender/pw_estudio
blender --command extension install-file -r user_default -e pw_estudio-1.0.0.zip
```

O painel "PW Estúdio" fica na barra lateral da vista 3D, com estes botões:
- **Importar GLB do Estúdio:** põe a cena a 15 (ou 30) quadros/s e importa.
- **Verificar para o Perfect World:**
  - até 65.536 vértices por malha (índice `u16`) e até 4 ossos por vértice (`A3DBVERTEX3`);
  - até 255 ossos por pele e nomes em GBK;
  - malha sem armadura tem de estar presa a um osso; vértice sem peso é erro;
  - imagem JPEG gera aviso;
  - ação com barra no nome é erro;
  - cena fora de 15/30 quadros/s gera aviso;
  - gancho fora de osso gera aviso.

  A forma de desenho de osso (a "Icosphere" do importador glTF) é ignorada.
- **Limitar a 4 ossos e normalizar.**
- **Novo gancho no osso ativo:** um vazio `HH_*`/`CC_*` preso ao osso.
- **Exportar GLB para o Estúdio:** recusa se a verificação tiver erro. Opções fixas: GLB, Y para cima, `extras` (os
  valores do Angelica voltam iguais), pele com 4 ossos, ações separadas e reamostradas, sem morph, sem luz nem câmera.

**Por que o FPS importa (medido):** com a cena a 24 quadros/s (o padrão do Blender), o Blender reamostra a animação
fora da grade, e a rotação perde até 5,1e-2; a 15/s, até 1,4e-4.

Prova: 3 NPCs reais por versão, baixados do painel com a ação mais longa, passaram pela extensão instalada (importar,
limitar, verificar, um gancho novo, exportar) e foram medidos por `importar::testes::medir_blender`
(`tools/blender/testar_pw_estudio.py` repete):

| item | resultado |
| :--- | :--- |
| esqueleto (`inicial`) | até 1,8e-5 |
| ganchos | até 7,2e-7; o novo `HH_teste` incluído |
| caixa das malhas | até 1,8e-6 |
| triângulos | iguais (numa malha, 2 degenerados a menos) |
| vértices | ±0–8, porque o Blender divide e junta por normal e UV |
| ação | mesmos quadros; posição até 2,6e-5, rotação até 1,4e-4 |

Pelo `pw-recursos`, os 6 viraram modelo novo com os mesmos ossos, malhas, ação e texturas, e o 1.5.5_0 com 20
ganchos (19 + 1). O canal de escala, que o Blender exporta em todo osso, vira um aviso só por animação.

**Habilidades (B301, S8):** três rotas, todas com `rascunho` opcional na leitura:
- `GET estudio/habilidades?busca=`: lista até 300 (id exato ou trecho do nome);
- `GET estudio/habilidades/{id}`: a ficha;
- `PUT estudio/habilidades/{id}/texto?rascunho=` `{nome?, descricao?}`: troca o texto.

O que a ficha junta:
- **números do servidor:** o `habilidades.json` embutido, extraído dos stubs do `gs`. O jogo é decidido pelo
  `elements.data` da versão (v7 = 1.2.6, v156/v159 = 1.5.5), porque o nome da versão de recursos é livre;
- **nome e descrição do cliente:** o `Configs\skillstr.txt` (fichas N×10 e N×10+1, `tabela_de_textos`).

A edição grava o `skillstr.txt` no rascunho como item `cliente`. Os números ainda só se leem: eles vivem no binário do
GS (embutidos) e no `elementskill.dll` do cliente.

No painel, a vista "Habilidades" tem busca, lista e ficha, com nome e descrição editáveis (a descrição em CRLF, como o
arquivo), os campos gerais, uma tabela por nível e o JSON completo. O limite da rota do texto é 32 KiB
(`seguranca.eh_texto_de_habilidade`).

**Números das habilidades (B302):** `PUT estudio/habilidades/{id}/numeros?rascunho=` `{campo: [um número por
nível]}` grava o item do servidor `catalogo\habilidades.json` (`Estudio::definir_numeros_da_habilidade`).
- **Onde vale:** a publicação o põe em `data/<realm>/catalogo/habilidades.json`, ao lado do `config`. É o arquivo que
  o GS já lê no lugar do embutido (`manager.rs:330-361`, B102). `governanca::caminho_no_realm` aceita
  `catalogo\<arquivo>` → `<config>/../catalogo/<arquivo>`.
- **Base:** o catálogo do realm, se existir; senão o embutido da versão.
- **Ficha:** mostra os números efetivos (rascunho → catálogo → embutido) e diz `origem_dos_numeros`.
- **Campos editáveis (`CAMPOS_EDITAVEIS`), marcados por quem os confere:**
  - **só o servidor:** `recarga_ms` (o cliente recebe a recarga por `SET_COOLDOWN`, `playerwrapper.cpp:170`) e `odio`;
  - **o cliente também confere, pelo `elementskill.dll` dele:** `mp` (`CheckSkillCastCondition`,
    `EC_HostPlayer.cpp:6097-6150`), `nivel_exigido`, `sp_exigido` e `dinheiro_exigido` (`LearnCondition`,
    `:6067-6093`). Mudar só o servidor desalinha.
- **Validação:** um número finito por nível; o arquivo editado tem de ler no `TabelaDeHabilidades::de_json` do GS.

**Montagens (`docker-compose.yml`):**
- o `pw-recursos` monta a pasta inteira do realm, com `PW_RECURSOS_REALMS` = `/realms/<realm>/config`;
- os mundos (`pw-world-126/155`) montam `data/<realm>/catalogo` em `/app/data/catalogo:ro`, ao lado do
  `/app/data/config` que o GS usa.

**A montagem nos mundos só vale depois de recriar os contêineres do GS.** Até lá, o GS segue com o embutido, e isso
fica a critério do Murillo.

Medido pelo `pw-recursos` em 5 `.smd` de NPC do 1.5.5, num rascunho de teste descartado:
- a 1ª ação de cada um (de 1 quadro), baixada e reimportada, sai byte a byte igual;
- a ação mais longa de cada um (442 quadros no total) sai "substituída" com o mesmo número de quadros, porque as
  chaves fora da grade de quadros são reamostradas (spec 03);
- a animação renomeada sai "adicionada";
- no 1.2.6, recusa com o motivo. A resposta traz
`texturas_gravadas: [{textura, acao}]`, com `acao` "adicionada", "substituída" ou "igual à do cliente (não gravada)".

Os arquivos são lidos com `Governanca::conteudos` (lote: o manifesto é lido e indexado uma vez; `conteudo` relê o
manifesto inteiro a cada chamada, o que levava a ficha do 1.5.5 a 50 s — agora 4,8 s, e 0,8 s no 1.2.6). No painel, o
seletor de perfil ganha "como modelo de voo — espada/asa/rodas". A ficha de `FLYSWORD_ESSENCE`/`WINGMANWING_ESSENCE`
abre o modelo já no perfil de voo, com o `fly_mode` do registro. Para cada nome que falta, o "Criar" do B280 cria a
ação.

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
(2 `servidor`, 2 `cliente`), publicados, carimbos e sequência iguais, o polígono novo vale no `distrito_em` do GS.
Destinos (B252): `.txt` gerado do `.sev` do 1.2.6; remover destino usado → 400; renomear e criar (id = maior + 1,
posição a 3 casas) → publicados, ids alinhados nos dois. `world_targets::testes_do_txt` (os `.txt` reais; empate 0,0625).
Alinhar (B253): `precinct.clt` com um distrito a menos enviado ao rascunho → `alinhamento` acusa → `alinhar` refaz só o
`precinct.sev` e o par volta a ser equivalente; de novo, nada a fazer. `clt::testes_do_servidor` (os 200 pares reais;
falha se passar dos 14 diferentes conhecidos). Loja (B254): a real do 1.2.6 (668 ofertas, ≥ 90% com o item no
`elements`) — preço e nome editados, clone e remoção, publicada: cliente e realm com o mesmo arquivo, carimbo igual, o
`GShopData` do GS lê o preço novo. `gshop_editavel::testes` (os 4 arquivos dos realms idênticos). Missões (B255): o
`tasks.data` real do 1.2.6 — 7.994 missões, 2.819 de topo, a 1173 com o NPC 3517 nomeado; id inexistente → 400. Editar
(B256): campo inexistente → 400; a ficha traz `textos` (B262) e `listas` (B263); pré-missão inexistente → 400;
pré-missão 1174 publicada (B263); 4 monstros → 400 e item entregue publicado (B264); prêmio em item publicado (B265); 5 diálogos na ficha, título de 64 → 400, título publicado (B266); clonar com id > 65535 ou com submissões → 400,
clone de uma missão de topo publicado (7.995 missões) com o nome da origem (B267); missão citada → 400, o clone sai
e volta a entrar com o mesmo id (B268); a 1173 (com submissões) é clonada e removida com a árvore (B269); função de opção desconhecida → 400, opção com
janela nova publicada (B270); nível mín. 5, exp 999 e nome novo na 1173 → visíveis no rascunho, o publicado não muda;
publicada: o realm recebe o arquivo e o leitor do GS lê as 7.994 com os valores novos, com a descrição longa nova
(B262). Comportamento (B258): 293
políticas no 1.2.6; uma usada tem os gatilhos e os monstros nomeados; busca pelo id do monstro acha a política; id
inexistente → 400. Modelos (B272): um `.ski` sintético (com `MOXB`) aparece na busca, a ficha traz versão,
textura, material e 1 triângulo, o GLB tem cabeçalho e tamanho certos; caminho que não é `.ski` → 400; a textura DXT1 na pasta `textures` (minúscula) é achada e vai no GLB como PNG
ligada ao material (B273); um `.smd` sintético com `.bon` de um osso vem primeiro na lista, a ficha traz o osso e a
pele, e o GLB tem `skin` com 2 juntas (o osso e o esqueleto) e pesos (B274); sem ações na ficha, e pedir uma → 400
(B276); um `.ecm` sintético vem primeiro e o diagnóstico acerta os três casos — combinada sem base, base ausente no
`.smd`, combinada que falta (B277); criar `普攻1落` clonando `普攻1起` no rascunho tira a falta do diagnóstico lido
com o rascunho, e nome repetido ou fora do GBK → 400 (B280); com `habitat=0` o diagnóstico de mascote pede só `_地面`
e acusa as faltas (B281); o GLB do `.ecm` sintético tem malha (a `caixa.ski` ao lado) e `skin` (B282); com `voo=1` o diagnóstico de voo do `.ecm` sintético é `asa`/`CC_chibang`, encaixe ausente (o `.bon`
sintético não tem ganchos) e nenhuma ação a conferir (B283). Tipos (B271): catálogo na ficha; operação nova acrescentada e removida; tipo que o arquivo
não usa → 400. Editar (B259): a ficha lista `skill` como editável; campo inexistente → 400; habilidade 4242/3 e
"só em combate" no rascunho, publicados no realm e relidos pelo leitor do GS. Estrutura (B261): fala trocada,
gatilho duplicado (id novo na resposta) e o original removido, publicados — o realm tem a mesma quantidade de
gatilhos, a cópia no lugar do original e a fala nova; operação inexistente → 400. Textos (B260): indexada com 2 e
sequencial achada pela busca; arquivo fora da lista → 400; editar e acrescentar na indexada, acrescentar na sequencial
e texto com aspas → 400; publicado não aplica nada no realm e o cliente baixa o arquivo novo. Controladores
(B247): 346 na a01, o 456 citado não se remove, `id` não muda, clone com id novo, publicado e lido pelo GS. Altura (B248):
sem `.hmap` responde `y` nulo; coordenada inválida 400. `pw-data-loader`: `npcgen_de_todos_os_mapas_volta_identico` (117
arquivos). Painel: `web-admin/backend/tests/teste_recursos.py` (10) — rodar no contêiner da imagem do painel com
`TEST_DATABASE_URL`/`TEST_REDIS_URL` (ver histórico B243).

### Windows 7 (B275)

O exe do Tauri não abria no Windows 7 sem mensagem nenhuma: a tabela de imports pedia
`api-ms-win-core-synch-l1-2-0.dll` (`WaitOnAddress`), `ProcessPrng`, `GetSystemTimePreciseAsFileTime`
(std do Rust ≥ 1.78, alvo `*-pc-windows-msvc` = Windows 10+), `combase.dll` e
`api-ms-win-core-winrt-error-l1-1-0.dll` (crate `windows` 0.6x do Tauri), além do WebView2 (parado na 109 no
Windows 7). Agora: std recompilada para o alvo `*-win7-windows-msvc` (`-Zbuild-std`, `RUSTC_BOOTSTRAP=1`,
componente `rust-src`), CRT estático (sem depender do Universal CRT/KB2999226), janela com
user32/gdi32/comctl32 v6/shell32/ole32, pasta escolhida pela `SHBrowseForFolderW`, manifesto
(`launcher.manifest`) com `supportedOS` do 7 ao 10 e `dpiAware`. Pânico vira caixa de erro (não some calado).
Confirmado pelo Murillo num Windows 7 real (2026-10-19). Conferir depois de compilar: nenhum import de `combase`, `api-ms-win-*` nem das três funções acima.

## 8. Falta

- Launcher: canal de teste para GM (instalar um rascunho antes de publicar), autoatualização do próprio
  launcher, instalador (o `.msi` do Tauri saiu com o Tauri no B275).
- ("Aguarda reinício" e reiniciar pelo painel: feitos no B249, spec 06.) Rascunho só de servidor continua gerando
  versão nova do cliente com o manifesto igual — **decisão do B250, mantido**: o launcher só baixa o manifesto e termina,
  e a versão é o que guarda o `publicada_em` do aviso de reinício e o que o reverter usa.
- Estúdio: telas próprias, `tasks` (a Loja Gold é do B254); no mapa, regiões (`[region]`) editáveis, nome de controlador (o
  `szName` é só leitura); decidir e aplicar o alinhamento dos 14 pares que diferem (ferramenta pronta, B253).
- Envio do cliente original pelo painel para importar no servidor externo (pasta enviada arquivo a
  arquivo, retomável — decisão B233).
- Pacotes novos de tempos em tempos (hoje a versão N reaproveita os da 0 e o resto vai por objeto).
- Troca de chave (`chaves.json` assinado pela anterior, §7.2).
