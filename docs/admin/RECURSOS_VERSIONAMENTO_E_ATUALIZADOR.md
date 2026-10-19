# Recursos do cliente, registro de alterações e atualizador próprio (B219)

Análise e plano, sem código. Responde a três perguntas do Murillo (2026-10-12):

1. Como editar modelos e animações — o usuário informa a pasta do cliente ou há coisa melhor?
2. Como versionar **toda** edição (`.data`, modelo, animação), registrando o que mudou e como.
3. Atualizador próprio no lugar do `.cup` oficial, já que teremos patcher e launcher nossos.

Complementa `EXTENSOES_E_EDITORES_FUTUROS.md` (B218), que trata dos editores de `.data`.
Estado de tudo aqui: `planejado`. **A §7 (B229) revê §2, §4 e §5; a §8 (B230) decide: cliente solto, instalado por zip, editores no painel web.**

## 1. Fatos (com evidência)

**Arquivos de dados são os mesmos no servidor e no cliente.** MD5 iguais (B219):
`data/realm_155/config/elements.data` = `…/1.5.5 BR/element/data/elements.data` (`44e36f71…`,
55.442.775 B); `tasks.data` idem (`ac773dd0…`, 104.198.815 B); `data/realm_126/config/elements.data`
= `F:\Games\perfectworld_126\element\data\elements.data` (`de943d87…`). Editar um deles é editar
**um arquivo que existe nos dois lados**. O cliente só registra o MD5 no log (`EC_Game.cpp:565-585`).

**Onde o cliente procura um arquivo.** `AFileImage::Init` (`AngelicaFile/Source/AFileImage.cpp:87-135`):
escolhe o pacote pelo **prefixo da pasta** (`AFilePackMan::GetFilePck`, `AFilePackMan.cpp:218-235`:
`models\…` → `models.pck`) e lê **do pacote primeiro** ("using package file first"); só vai ao disco
se a entrada não existir no pacote ou se o pacote não abrir. Consequências:

- Arquivo solto **não substitui** uma entrada que já está num pacote que abre. Para trocar um
  modelo dentro do `gfx.pck` (339 MB) é preciso **regravar a entrada no pacote**.
- `models.pck`+`.pkx` (3,2 GB) e `litmodels.pck`+`.pkx` (2,9 GB) não abrem no binário (`int` de 32 bits,
  B6a/B9f) — por isso os arquivos soltos funcionam **para eles**. O README do `pw-pck-extract`
  generalizava ("sempre procura em disco antes do pacote"); corrigido no B219.
- Isso é o fonte. **O binário é o juiz**: a ordem precisa ser conferida no `elementclient.exe` BR
  (2569) antes do primeiro patch real (teste: entrada alterada solta + a mesma no `gfx.pck`).

**O formato dos pacotes já tem escrita.** `AFilePackage::AppendFile` / `ReplaceFile` / `RemoveFile` /
`SaveEntries` (`AFilePackage.cpp:995-1275, 1825`); o atualizador oficial usa exatamente isso
(`CNewSkin/packdll/PackDLL.cpp:344-381, 439`). Nosso leitor (`tools/pw-pck-extract/pw_pck_extract.py`)
já tem as constantes e os structs calibrados no binário (`FILEHEADER` 272 B, `FILEENTRY` 276 B).

**Modelos e animações: formatos e quem lê/grava.** O fonte do cliente tem leitor **e** escritor de todos:

| arquivo | o que é | cabeçalho | ler/gravar no fonte |
| :--- | :--- | :--- | :--- |
| `.ecm` | modelo do jogo (texto: `Version: %d`, `SkinModelPath:`, ações, filhos, efeitos) | texto | `GfxCommon/EC_Model.cpp:146-151`, `LoadData` `:4308`, `Save` `:4909` |
| `.smd` | modelo com pele | `ASMD` v8 | `Angelica3D/Header/A3DSkinModel.h:32-33`; `A3DSkinModel::Save` `A3DSkinModel.cpp:302` |
| `.bon` | esqueleto | `ASKE` v6 | `A3DSkeleton.cpp:34-35`, `Save` `:585` |
| `.ski` | malha da pele | `ASKI` v9 | `A3DSkin.cpp:47-48`, `Save` `:523` |
| `.stck` | trilhas de animação | `STCK` v1 | `A3DTrackMan.cpp:29-30`, `A3DSklTrackSet::Save` `:281` |
| `.phy` | física da pele | `APHY` v1 | `A3DSkinModel.h:36-37`, `SavePhysique` `:782` |
| `.gfx`, `.att`, `.dds` | efeitos, ataque, texturas | — | `gfx.pck`, `textures.pck`, `surfaces.pck` |

Pacotes do cliente BR (MB): building 1.761, configs 7, facedata 195, fonts 4, gfx 339, grasses 31,
interfaces 8, litmodels 2.048+936, loddata 369, models 2.048+1.217, sfx 408, shaders 11,
surfaces 249, textures 251, trees 22.

**O atualizador oficial** (`CNewSkin`): `config/element/version.txt` + `ec_patch_<de>-<para>.cup`
com `.MD5` (`PackUPdater.cpp:32-36`). O `.cup` é um pacote Angelica com `v-<n>.inc` (linha
`<tag> <de> <para>`, depois `<md5> <caminho em base64 por segmento>`; md5 começando com `-` =
apagar), `pid` e os arquivos em `element/…` (`CupReader.cpp:206-330`). O `.inc` é **assinado**
(RSA sobre MD5, `-----BEGIN ELEMENT SIGNATURE-----`) e conferido com uma chave pública embutida
(`patchlib/EC_Signature.cpp:23-70`; `Update.cpp:1437, 2638, 2856, 3327`). Não temos a chave privada.

**O que temos hoje:** `tools/pw-patch-tool` (`scan` com SHA-256; `create-patch` **simulado** — não
compara as pastas e grava tamanho 14,5 MB, 8 arquivos e o SHA-256 de arquivo vazio,
`main.rs:100-125`); `tools/pw-pck-extract` (só lê).

## 2. Editar modelos e animações: de onde vem o cliente

### Recomendação: um **cliente de referência** por linha de cliente, registrado uma vez

O usuário (operador) informa a pasta do cliente **uma vez por linha** (`1.5.5`, `126`), numa
importação feita no host — não a cada edição, e nunca a pasta de um jogador.

```text
pw-recursos importar --linha 1.5.5 --pasta "F:\PW\1.5.5\1.5.5 BR\Perfect World 1.5.5 BR\element"
```

A importação lê cada arquivo solto e **cada entrada de cada `.pck`** e grava o **índice da base**:
caminho normalizado (minúsculas, `\`), origem (`solto` ou nome do pacote), tamanho, SHA-256 e MD5.
O conteúdo não é copiado: a pasta fica montada **só para leitura** em `data/clientes/<linha>/original`
e o que for editado é extraído dela sob demanda.

Por que não "o usuário informa a pasta" a cada edição:

| alternativa | problema |
| :--- | :--- |
| Pasta local de quem edita | o painel roda no Docker do servidor e o navegador não lê pastas; cada GM tem um cliente diferente (talvez já modificado) — o patch não teria base conhecida |
| Enviar o cliente pelo navegador | 7+ GB; a CSP e o limite de 8 KiB do corpo da API existem por um motivo |
| File System Access API (Chromium) | serve para **escolher o arquivo editado** a enviar, não como base |

A base precisa ser **uma só, com hash de tudo**: é dela que saem os pacotes de atualização e a
verificação dos clientes dos jogadores.

### Fluxo de edição (o painel não é editor 3D)

1. **Exportar o conjunto** — pelo painel, um modelo e tudo que ele referencia (`.ecm` → `.smd` →
   `.bon`/`.ski`/`.stck`/`.phy` → texturas e `.gfx`), lido da versão atual da linha (base + alterações
   publicadas), num `.zip`.
2. **Editar fora** — em ferramenta 3D. Hoje não existe conversor nosso; o caminho é portar os
   leitores/escritores do `Angelica3D` (tabela do §1) para um importador/exportador do Blender
   (fase F7). Trocar só textura `.dds` ou um `.gfx` já funciona sem conversor.
3. **Enviar** — o painel recebe o `.zip` (upload em blocos, fora do limite de 8 KiB das rotas de
   comando) e o serviço de recursos **valida**: identificador e versão do cabeçalho (`ASMD` 8,
   `ASKE` 6, `ASKI` 9, `STCK` 1, `APHY` 1, `.ecm` `Version:` em GBK), leitura completa pelo leitor
   portado **até o último byte**, referências internas apontando para arquivos que existem (na base
   ou no mesmo envio), caminho dentro de uma pasta de pacote conhecida.
4. **Vira uma alteração** em rascunho no registro (§3), com o resumo do que mudou.
5. **Publicar** gera uma versão da linha (§3) e o atualizador a entrega (§4).

Quem faz isso é um serviço novo, **`pw-recursos`** (Rust), não o GS: o servidor de mundo não lê
recurso de cliente. Os `.data` continuam validados pelo GS (o leitor de dados é dele) e entram no
**mesmo registro**.

## 3. Registro de alterações (versionamento)

Objetivo: para qualquer versão, saber **o que mudou, como, quem, quando e por quê**, poder voltar
atrás, e gerar daí o pacote de atualização. Modelo: **objetos por conteúdo + livro de alterações
só de acréscimo**, no PostgreSQL que o painel já usa. Git não serve: binários de GB e consultas do painel.

### Armazenamento

- **Objetos:** `data/recursos/objetos/<sha256[0..2]>/<sha256>.zst` — cada versão de cada arquivo
  tocado, comprimida, imutável. A base não é copiada: o índice aponta para o cliente de referência.
- **Tabelas** (migração nova; nomes provisórios):

| tabela | colunas principais |
| :--- | :--- |
| `recursos_linhas` | `id` (`1.5.5`, `126`), versão do cliente (build, `elements` v156…), realms que a usam, caminho da referência, `sha256` do índice da base |
| `recursos_base` | `linha`, `caminho`, `origem` (solto/pacote), `tamanho`, `sha256`, `md5` |
| `recursos_alteracoes` | `id`, `linha`, `autor` (conta GM), `criada_em`, `descricao`, `estado` (`rascunho`/`publicada`/`descartada`), `operacao_id` (idempotência, como `comandos_administrativos`), `reverte` (id, quando for desfazer outra) |
| `recursos_alteracao_itens` | `alteracao`, `caminho`, `destino` (`cliente`, `servidor`, `ambos`), `pacote` (`gfx.pck`/`solto`/`config`), `acao` (`adicionar`/`substituir`/`remover`), `sha256_antes`, `sha256_depois`, `tamanho`, `resumo` (JSON) |
| `recursos_versoes` | `linha`, `numero` (1, 2, 3…), `alteracoes` incluídas, `manifesto_sha256`, `publicada_em`, `publicada_por`, `minima` (versão mínima aceita pelo launcher) |

- **O que foi modificado** fica em `resumo`, conforme o tipo:
  - `.data`: diferença **por campo**, feita pelo GS com o layout — `{tabela, id, campo, antes, depois}`
    (`elements`), `{missao, campo, antes, depois}` (`tasks`), `{mapa, area, campo, antes, depois}`
    (`npcgen`). Destino `ambos` para `elements`/`tasks` (MD5 iguais, §1), `servidor` para `npcgen`.
  - Modelo/animação: cabeçalho e números do formato (ossos, malhas, trilhas, quadros), e as
    referências que mudaram no `.ecm`.
  - Textura e outros: tamanho e dimensões quando se souber ler; senão só o hash.
- **Regras:** linha publicada nunca é alterada nem apagada. Desfazer = nova alteração com
  `reverte` (os objetos antigos continuam guardados). Um caminho só pode estar em um rascunho por
  vez (evita duas edições concorrentes do mesmo arquivo). A versão N da linha é sempre
  **base + alterações publicadas até N**, em ordem.
- **Exportação para auditoria:** `pw-recursos historico --linha 1.5.5 [--desde N]` gera JSON/Markdown.

### Ligação com os realms

Publicar uma versão que muda `elements`/`tasks`/`npcgen` também gera o arquivo novo do realm
(`data/<realm>/config/…`, troca atômica do `EXTENSOES…` §2) e marca `aguarda_reinicio`. Ordem
operacional: publicar → launcher atualiza os clientes (versão mínima sobe) → reiniciar o realm.
O servidor **não tem como saber** a versão do cliente (o cliente não manda o MD5); quem burlar o
launcher joga dessincronizado — risco a aceitar ou a resolver com uma checagem no login (fora do
protocolo original; decisão do Murillo).

## 4. Atualizador próprio (no lugar do `.cup`)

O `.cup` fica só como referência: a assinatura dele exige uma chave privada que não temos, os nomes
vêm em base64 por segmento, o hash é MD5, e o pacote é "de A para B" (um por par de versões, sem
retomar download nem reparar). Proposta:

### No servidor (arquivos estáticos, servidos por HTTP)

```text
atualizacao/<linha>/versao.json          {"versao": 12, "minima": 11, "manifesto": "<sha256>", "launcher": {...}}
atualizacao/<linha>/versao.json.sig      assinatura Ed25519 (chave nossa; a pública vai embutida no launcher)
atualizacao/<linha>/manifestos/12.json   caminho → {sha256, tamanho, pacote|solto, acao}  (estado completo da versão)
atualizacao/<linha>/mudancas/11-12.json  só o que mudou entre duas versões (atalho; derivado do registro)
atualizacao/<linha>/objetos/<sha256>.zst conteúdo endereçado por hash: baixado uma vez, reaproveitado entre versões
```

Gerado pelo `pw-recursos publicar`; o manifesto e as mudanças saem do registro (§3), não de comparar
pastas. HTTP comum com `Range`: download retomável.

### No cliente do jogador (launcher)

1. Lê `element/pw-atualizacao.json` (linha, versão aplicada).
2. Baixa `versao.json` e a assinatura; confere com a chave pública embutida.
3. Baixa `mudancas/<local>-<atual>.json` (ou o manifesto completo, se o salto for grande ou o local
   estiver corrompido).
4. Para cada item: baixa o objeto, confere o SHA-256, prepara em `element/.atualizacao/`.
5. Aplica numa ordem que retoma do ponto onde parou:
   - pacote que o cliente abre (`gfx.pck`, `configs.pck`…) → **regravar a entrada no `.pck`**
     (port do `AFilePackage::ReplaceFile`/`AppendFile`/`RemoveFile` + `SaveEntries`; fase F3);
   - pacote que o cliente não abre (`models`, `litmodels`, > 2 GB) e arquivos soltos (`data\elements.data`…)
     → arquivo solto;
   - remover → apaga o solto ou remove a entrada do pacote.
6. Confere os hashes do que aplicou e grava a versão local.
7. **Reparar:** confere todos os arquivos com o manifesto completo e baixa o que divergir.
8. Também atualiza o próprio launcher, `serverlist.txt` (UTF-16LE com BOM, B10) e o `.bat` com
   `logiccheck:0`.

| | `.cup` oficial | atualizador próprio |
| :--- | :--- | :--- |
| assinatura | RSA com chave que não temos | Ed25519 com chave nossa |
| hash | MD5 | SHA-256 (+ MD5 no índice, para comparar com o log do cliente) |
| unidade | pacote por par de versões | objeto por conteúdo (sem duplicar entre versões) |
| retomar / reparar | não | sim (Range; manifesto completo) |
| origem do conteúdo | pastas comparadas à mão | o registro de alterações (§3) |
| várias linhas (126, 155) | não | `atualizacao/<linha>/` |

## 5. Fases propostas (fila depois da reforma)

| fase | entrega | depende de |
| :--- | :--- | :--- |
| F1 | `pw-recursos importar`: leitor de `.pck` em Rust (port do `pw-pck-extract`) + índice da base no banco | — |
| F2 | registro de alterações (migração + objetos) e "histórico" no painel, só leitura | F1 |
| F3 | escritor de `.pck` com ida e volta (pacote regravado sem mudança = idêntico byte a byte) e o teste no binário da ordem pacote × solto | F1 |
| F4 | `pw-recursos publicar`: manifestos, mudanças, objetos, assinatura | F2 |
| F5 | launcher (Rust, Windows): verificar, aplicar, reparar, atualizar a si mesmo; substitui o `create-patch` simulado | F3, F4 |
| F6 | painel: exportar conjunto de modelo / enviar com validação de cabeçalho; troca de textura e `.gfx` | F2 |
| F7 | leitores/escritores dos formatos do `Angelica3D` em Rust + importador/exportador do Blender | F6 |
| — | editores de `.data` (B218) passam a gravar no mesmo registro | F2 |

## 6. Decisões abertas (do Murillo)

1. Onde hospedar `atualizacao/`: no mesmo Docker (serviço HTTP só de arquivos) ou fora.
2. Quem guarda a chave privada de assinatura e como (fora do repositório, como `docker/.env`).
3. Linhas de cliente: `1.5.5` e `126` só, ou também o BR novo (build 2591, `elements` v181, sem servidor ainda).
4. Manter os `.pck` (patcher regrava entradas, F3) ou distribuir tudo como soltos (patcher mais simples,
   mas só vale onde o pacote não tem a entrada — §1).
5. Checagem de versão do cliente no login (fora do protocolo original) ou aceitar o risco.

## 7. Revisão de 2026-10-13 (B229): cliente só com arquivos soltos, launcher com Estúdio

Proposta do Murillo: um **launcher do pw-universal** onde se escolhe a versão, que baixa só a pasta
`element` dessa versão e depois busca as atualizações no servidor do Docker; cliente editável, de
preferência **sem `.pck`**; e a dúvida entre editor completo no painel web ou num aplicativo Windows.
Esta seção **substitui** o que divergir nos §2, §4 e §5 (que assumiam manter os pacotes).

### 7.1 Cliente sem `.pck`: viável, e suportado pelo próprio cliente

- **Modo oficial de arquivos separados.** Na partida o cliente abre um pacote por pasta da lista
  `g_szPckDir` (`ElementClient/CCommon/elementpckdir.h:4-21`); se um falta, mostra erro e **sai** — a não
  ser que a linha de comando tenha a chave **`sepfile`** (`ElementClient.cpp:1263-1276`;
  `CECCommandLine::GetSupportSeperateFile`, `EC_CommandLine.cpp:138-141`, chave sem valor). Sem o
  pacote, `GetFilePck` não acha pacote para a pasta e `AFileImage::Init` lê do disco
  (`AFileImage.cpp:94-135`).
- **O binário tem a chave:** `sepfile` está no `elementclient.exe` BR 2569 (UTF-16, `0x9cb268`, junto de
  `rendernofocus` e `nocheck`) e no do 1.2.6 (ASCII, build de 2008). Falta o teste em jogo (§7.5).
- **Já funciona na prática para a maior parte:** o cliente BR roda com `models` (139.333 entradas) e
  `litmodels` (113.862) soltos desde o B6a. O resto soma ~58 mil entradas (gfx 27.860, building 15.478,
  surfaces 9.503, sfx 2.890, textures 1.748, configs 111 e os pequenos).
- **Desempenho — o que muda:**

| | pacote | solto |
| :--- | :--- | :--- |
| abrir um arquivo | busca no índice em memória + `seek` num arquivo já aberto | um `fopen` por arquivo (o antivírus examina na 1ª abertura) |
| ler | zlib a descompactar (CPU) | leitura direta, sem descompactar |
| arquivo que não existe | consulta o índice e **também** tenta o disco (`AFileImage.cpp:103-106`) | tenta o disco — mesmo custo |
| espaço | comprimido | ~1,5–2,2× maior: gfx 339→739 MB, building 1.761→2.478, textures 251→545, surfaces 249→501 (soma dos tamanhos reais no índice) — ~+2 GB no total |
| modelos | — | carregados em segundo plano (`EC_ModelAsynLoadHelper`), o que esconde o custo do `fopen` |

  Expectativa: **sem perda perceptível** em SSD (troca a CPU de descompactar pela abertura de arquivo;
  o BR já faz isso com 253 mil arquivos). Riscos: HD mecânico e antivírus na primeira carga de cada
  mapa. Isso se **mede** antes de decidir de vez (§7.5).
- **O que melhora para nós:** sem escritor de `.pck` (a fase F3 some); patch = trocar/apagar arquivo;
  o editor lê e grava arquivos comuns; testar uma edição = abrir o cliente; o problema do pacote
  > 2 GB deixa de existir; a ordem pacote × solto (§1) deixa de importar.
- **Proposta:** a linha do pw-universal é **toda solta** e sempre inicia com
  `elementclient.exe game:cpw console=1 logiccheck:0 sepfile` (mais `startbypatcher` se o binário o
  exigir); nenhum `.pck` na pasta.

### 7.2 Assinaturas de versão (liberdade dada pelo Murillo)

- Chave **Ed25519** gerada no servidor, guardada como segredo do Docker
  (`docker/segredos/assinatura_atualizacao.key`, fora do git, montada **só** no serviço `pw-recursos`).
- Quem assina: o `pw-recursos publicar` (§3/§4), ao gerar `versao.json` e cada manifesto.
- Chave pública **embutida no launcher**; troca de chave por uma lista de chaves assinada pela anterior
  (`chaves.json`), sem reinstalar o launcher.
- O banco guarda versão, manifesto e assinatura (`recursos_versoes`); o mesmo serviço serve
  `atualizacao/<linha>/…` por HTTP.

### 7.3 Editor completo de `elements` e recursos: painel web × aplicativo Windows

Problemas de fazer tudo no **painel web**:

1. **Os arquivos do cliente não estão no servidor** e o navegador não lê pastas (só a File System
   Access API do Chromium, com permissão por sessão — inviável para ~310 mil arquivos).
2. **Volume:** 7–10 GB por linha; enviar pelo navegador não escala.
3. **Visualizar modelo** exige portar os formatos do `Angelica3D` para JavaScript/WebGL (sem
   biblioteca: a CSP é `self`) — o maior trabalho, e preso ao navegador.
4. **Testar** uma edição exige abrir o cliente de qualquer jeito, e o navegador não faz isso.
5. Dois lugares lendo e gravando o mesmo formato (Rust no servidor e JS no painel) = duas
   implementações para manter iguais — contra a regra "um caminho de escrita por layout".

O que o web faz bem: `elements.data`/`tasks.data`/`npcgen.data` **estão no servidor** (MD5 iguais aos do
cliente, §1); editar só valores de dados no navegador seria viável, mas sem o resto acima.

**Recomendação: launcher pw-universal com dois modos, e o servidor como fonte da verdade.**

| peça | faz | onde |
| :--- | :--- | :--- |
| **Launcher — Jogar** | escolhe a linha (1.2.6, 1.5.5 BR…), baixa a `element` dessa versão, atualiza, verifica/repara, abre o cliente com `sepfile` | Windows, Rust |
| **Launcher — Estúdio** (só conta GM) | abre a cópia de trabalho local da linha; editor de `elements`/`tasks`/`npcgen` e de recursos (modelo, animação, textura, `.gfx`); **Testar** abre o cliente com a edição; **Enviar** manda as alterações ao servidor | o mesmo aplicativo; usa os crates Rust do projeto (`pw-data-loader` + escritores) |
| **`pw-recursos`** (Docker) | repositório por linha (versão 0, registro de alterações, objetos), validação final (o mesmo código Rust), publicação assinada, arquivos de atualização | servidor |
| **Painel web** | contas, personagens, mapas, rates, templates (já feito) + **governança do conteúdo**: histórico, diferenças, publicar, reverter, versão mínima | navegador |

Por quê: o Estúdio acessa direto a pasta do cliente que o próprio launcher criou (o usuário não informa
caminho), roda o cliente para testar e reaproveita **o mesmo código Rust** que o servidor usa para
validar. O servidor continua sendo a única fonte da versão publicada: cada envio é uma alteração no
registro com o hash do arquivo de onde partiu — se ele mudou no meio, conflito, e ninguém sobrescreve
ninguém. O "cliente de referência" do §2 vira a **versão 0 da linha**, importada uma vez no servidor.

Interface do aplicativo — a decidir: **Tauri** (Rust + webview: mesma aparência do painel, WebGL para o
3D) ou **egui/wgpu** (só Rust, 3D nativo). Recomendação: Tauri — reaproveita o visual do painel, e o
lado Rust chama os crates do projeto direto.

### 7.4 Fases revistas

| fase | entrega |
| :--- | :--- |
| G0 | teste do `sepfile` em jogo e medição (§7.5) — decide 7.1 |
| G1 | `pw-recursos`: importar a versão 0 de cada linha (soltos + extração dos `.pck` uma vez), registro de alterações, objetos, publicar com assinatura |
| G2 | launcher **Jogar**: escolher linha, baixar, atualizar, verificar/reparar, `serverlist.txt`, abrir com `sepfile` |
| G3 | painel web: histórico, diferenças, publicar/reverter, versão mínima |
| G4 | Estúdio: editor de `elements` (escritor + ida e volta byte a byte; ícones e textos da cópia local) e envio |
| G5 | Estúdio: `tasks` e `npcgen` |
| G6 | Estúdio: recursos — trocar textura/`.gfx`, depois visualizar e importar/exportar modelo e animação (port do `Angelica3D`) |

Sai a F3 (escritor de `.pck`); a importação usa só a leitura (`pw-pck-extract`, a portar para Rust).

### 7.4b Decisões de 2026-10-13

- **Teste G0:** Claude prepara a cópia sem pacotes (`F:\PW\1.5.5\1.5.5 BR sepfile\element`, `models`/`litmodels`
  por link físico do BR, o resto copiado, os outros `.pck` extraídos) e o `.bat` com `sepfile`; o Murillo
  joga e mede.
- **Aplicativo Windows:** Tauri (B234); trocado por Win32 nativo no B275 — o Tauri 2 exige Windows 10 e o launcher precisa abrir no Windows 7 (spec 07 §5).
- **Ordem:** servidor (`pw-recursos`, G1) → Estúdio com o editor de `elements` (G4) → launcher Jogar (G2);
  a governança no painel (G3) acompanha o G1/G4.
- **Onde fica o editor de dados: em análise.** O Murillo prefere tudo no painel web; o limite são os
  arquivos do cliente (ícones, textos, modelos para pré-visualizar) e, no `npcgen`, o servidor ter de
  responder com os dados do mapa (terreno, criaturas do `elements`) para posicionar. A decidir depois do G1.

### 7.5 Teste antes de decidir (roteiro para o Murillo)

1. Copiar a pasta `element` do BR para uma pasta de teste.
2. Extrair `gfx.pck`, `textures.pck`, `surfaces.pck`, `sfx.pck`, `building.pck` e os pequenos com
   `tools/pw-pck-extract` (`get-all`) e **remover** os `.pck`/`.pkx`.
3. Iniciar com `elementclient.exe game:cpw console=1 startbypatcher logiccheck:0 sepfile`.
4. Esperado: sem a mensagem de pacote danificado; login, seleção, entrada no mapa 161 com efeitos,
   texturas e sons. Medir o tempo até a seleção e até o mundo, e o FPS parado na cidade — e o mesmo no
   cliente com pacotes. Sem `sepfile` o cliente deve parar na mensagem de pacote danificado (confirma
   que é a chave que manda).

**Cópia preparada (2026-10-13):** `F:\PW\1.5.5\1.5.5 BR sepfile\element` — 60 entradas copiadas do BR,
`models`/`litmodels` por link físico (nada é gravado neles), nenhum `.pck`/`.pkx`; extraídos inteiros configs 109,
facedata 2.047, fonts 2, gfx 27.858, grasses 3.332, interfaces 2.640 (por cima dos soltos, como o pacote vence
hoje), loddata 4.135, sfx 2.888, shaders 520, surfaces 9.501, textures 1.746, trees 326; building 15.475/15.476 —
`building\textures\t\1790.dds` falha no zlib (`incorrect data check`) **dentro do próprio pacote**, então o
cliente original também não a lê. Arquivos: `elementclient_sepfile.bat` (teste) e
`elementclient_sem_sepfile_controle.bat` (deve parar na mensagem de pacote danificado).

### 7.6 Riscos a registrar

- Distribuir o cliente original inteiro pelo nosso servidor (conteúdo de terceiros) — decisão do
  projeto, fora do técnico.
- Primeira instalação de 7–10 GB por linha: objetos comprimidos (zstd) no transporte devolvem um
  tamanho perto do dos pacotes.
- ~310 mil arquivos: o launcher não calcula o hash de tudo a cada partida — guarda tamanho + data de
  cada arquivo e só recalcula o que mudou (e tudo no "reparar").

### 7.7 1º teste do `sepfile` (Murillo, 2026-10-13) e a página de código

- **Sem `sepfile`:** parou em `configs.pck` com a mensagem de pacote danificado — como previsto (§7.5).
- **Com `sepfile`:** passou dos pacotes (`AF.log`: "Can not open package" em todos, sem sair) e parou em
  `AFileImage::Can not open file [Models\Players\形象\白虎\白虎.ecm] from disk!`. A janela mostrava
  `Models\Players\ÐÎÏó\°×»¢\°×»¢.ecm`.
- **Causa:** o cliente monta o caminho com os **bytes GBK** que vêm dos dados e abre o solto com `fopen`
  ANSI (`AFileImage.cpp:127`); o Windows converte os bytes pela **página de código ANSI do sistema** — a
  desta máquina é **1252** (`HKLM\…\Nls\CodePage\ACP`). `形象` (`D0 CE CF F3`) vira `ÐÎÏó`. O
  `pw-pck-extract` gravava o nome em chinês (`cp936`, com `�` onde não decodificava): arquivo
  invisível para o cliente. Dentro do pacote isso não acontece — a busca é pelos bytes, em memória.
  (Também: os `models`/`litmodels` soltos do BR tinham nomes em chinês e o `白虎.ecm`, que mora no
  `.pkx`, nem tinha sido extraído — o BR funciona porque usa o pacote.)
- **Correção (B229):** o extrator guarda os bytes do nome e grava o solto com `bytes.decode("mbcs")` —
  a mesma conversão `CP_ACP` do `fopen` (`nome_no_disco`, `PW_NOMES=cp936` volta ao nome chinês para
  inspeção). A cópia de teste foi refeita com os 15 pacotes por esse caminho.

**Consequência para o cliente só com soltos (revê a §7.1):** o nome correto no disco **depende do Windows
de cada jogador**. ACP 1252 (português, inglês…) → `ÐÎÏó`; ACP 936 (chinês) → `形象`; ACP 1251 (russo),
932 (japonês) etc. → outros nomes, e numa ACP de dois bytes alguns nomes GBK podem nem ter conversão. E um
byte final de caractere GBK pode cair em `0x7C` (`|`, proibido em nome de arquivo) ou `0x5C` (`\`, vira
separador de pasta). Por isso:

1. O **manifesto guarda o nome em bytes GBK** (hex), nunca um nome Unicode pronto.
2. O **launcher calcula o nome no disco na máquina do jogador** com `MultiByteToWideChar(CP_ACP)` e
   recusa instalar (avisando) se a ACP não converter algum nome.
3. Trocar a página de código do Windows depois de instalado = "reparar" (renomear tudo).
4. Alternativa que elimina o problema: **manter pacotes só para os arquivos de nome não ASCII**, ou
   manter os pacotes e soltar só o que se edita — volta a exigir o escritor de `.pck` (F3). Decidir
   depois do 2º teste.

**2º teste (Murillo, 2026-10-13 20:44):** com os nomes pela ACP, o cliente passou dos pacotes, leu
`elements.data`/`tasks.data` (MD5 iguais aos do servidor), carregou o mundo da tela de login e caiu antes
da janela de login (`ec_build2569.dmp`). Os avisos de antes da queda — `Gfx\gfxlist.txt`, os cursores com
nome chinês e `loddata\Login\olm\1.olm` (`CECWorld::LoadWorld` linha 617) — aparecem **iguais** no BR com
pacotes, então não são a causa. O minidump: `ACCESS_VIOLATION` lendo o endereço 0 em
`ELEMENTCLIENT.EXE+0x3DD1DD` (`mov edi,[ecx+0x14]; cmp byte [edi],0`, `edi = 0`), chamada da abertura da
tela de login (`Win_Login`, `Win_LoginServerListButton`, `Win_LoginVersion`) — no fonte,
`CDlgLoginServerList` → `CECServerList::Instance().FindGroup(iServer, true)` (`DlgLoginServerList.cpp`),
que lê `GetGroup(0)` (`EC_ServerList.cpp:289-300`). Os grupos só nascem de linhas **sem tabulação** da
`userdata\server\serverlist.txt` (`:70-158`), e a nossa tem só linhas de servidor: vetor de grupos vazio →
endereço 0. Correção na cópia de teste: linha de grupo `pw-universal` no topo (original em
`serverlist.txt.antes`). **O launcher deve gerar a `serverlist.txt` sempre com uma linha de grupo.**
Falta explicar por que o BR com pacotes, com o mesmo arquivo, não cai — hipótese: algum arquivo que antes
não era achado agora é, e ativa a janela nova da lista de servidores (a confirmar).

## 8. Decisão de 2026-10-14 (B230): cliente solto, distribuído por zip

**3º teste (Murillo):** com a linha de grupo na `serverlist.txt`, o cliente sem `.pck` (`sepfile`) e o BR com
pacotes **funcionaram igual — mesma memória e mesmo FPS**. A §7.1 está confirmada em jogo; a F3 (escritor de
`.pck`) sai de vez.

**Decisão:** a linha do pw-universal é **toda solta**. A instalação é um **zip da pasta `element`** hospedado
no servidor; o launcher baixa, extrai e apaga o zip. As atualizações seguintes vêm arquivo a arquivo, pelo
registro de alterações (§3/§4) — não um zip novo a cada mudança.

**O zip guarda o nome em bytes GBK, não o nome desta máquina.** O nome certo no disco depende da página de
código de cada jogador (§7.7). Então:

- a entrada do zip leva os **bytes GBK originais** do nome (sem a marca UTF-8 do zip; zip64, porque passa de
  4 GB) e o manifesto repete os bytes em hex com SHA-256 e tamanho;
- o **launcher extrai com o próprio código** (Rust: `zip` lê o nome cru) e converte cada nome com
  `MultiByteToWideChar(CP_ACP)` — um descompactador comum (Windows Explorer, 7-Zip) daria nomes errados;
- o zip é gerado pelo `pw-recursos` a partir dos `.pck` originais (bytes do índice) e dos soltos da instalação,
  nunca de uma pasta já extraída numa máquina (os nomes dela já são da ACP dela);
- de quebra, os cursores com nome chinês da instalação BR (`Cursors\七夕鼠标.cur`…), que hoje falham no
  `EC.log` dos dois clientes, passam a ser achados.

**Também entra no zip/launcher:** `serverlist.txt` com linha de grupo (§7.7, 2º teste), o `.bat`/atalho com
`sepfile`, sem nenhum `.pck`.

### 8.1 Onde fica o editor (revisão da §7.3)

A objeção principal ao painel web era "os arquivos do cliente não estão no servidor". **Com o zip, estão**: o
servidor guarda a versão 0 e todas as alterações de cada linha. O que muda:

| editor | onde | por quê |
| :--- | :--- | :--- |
| `elements`, `tasks`, `gshop` | **painel web** | o servidor tem o arquivo (MD5 igual ao do cliente), os ícones e os textos; a gravação é Rust no servidor (um caminho só); o GS valida |
| `npcgen` | **painel web** | precisa do mapa — terreno, posições, criaturas — que o GS já tem carregado (`.hmap`, `npcgen.rs`); a tela pede ao GS pelo canal |
| texturas, `.gfx`, sons | **painel web** (trocar arquivo) | enviar o arquivo, validar o cabeçalho, registrar |
| modelos e animações | **fora do painel**: exportar/importar pelo Blender (port dos formatos do `Angelica3D`) e enviar pelo painel | visualizar 3D no navegador seria escrever um motor; o Blender já é o editor |
| **testar a edição** | **launcher, canal de teste** | o GM aponta o launcher para a versão-rascunho da linha e abre o próprio cliente com a mudança; publicar = promover o rascunho |

Resultado: **não há "Estúdio" no launcher.** O launcher fica só com Jogar + canal de teste (para contas GM);
toda edição e governança ficam no painel web, e o pesado de 3D vai para o Blender.

### 8.2 Próximos passos (substitui a §7.4)

| passo | entrega |
| :--- | :--- |
| P1 | `pw-recursos` (Rust, Docker): importar a versão 0 de cada linha (`1.5.5`, `126`) dos `.pck` + soltos; gerar o zip com nomes GBK e o manifesto; tabelas do registro de alterações (§3); publicar com assinatura Ed25519 (§7.2); servir `atualizacao/<linha>/…` |
| P2 | launcher (Tauri): escolher a linha, baixar o zip (retomável), conferir SHA-256, extrair com a ACP, apagar o zip, aplicar atualizações por arquivo, reparar, `serverlist.txt`, abrir com `sepfile`; canal de teste para GM |
| P3 | painel: governança (histórico, diferenças, publicar, reverter, versão mínima) |
| P4 | painel: editor de `elements` (escritor com ida e volta byte a byte; destino `ambos`: realm + cliente) |
| P5 | painel: `tasks`, `gshop`, `npcgen` (vista do mapa pelo GS) |
| P6 | painel: trocar textura/`.gfx`/som; depois Blender ↔ formatos de modelo e animação |

Falta conferir o `sepfile` no cliente 1.2.6 (a chave está no binário, não foi testada em jogo).

### 8.3 Visualizador e editor 3D no próprio painel (análise de 2026-10-14)

Pergunta do Murillo: dá para o painel visualizar e editar modelos, texturas e animações sem Blender?

**O formato ajuda.** Vértice da pele = posição, normal, UV, **3 pesos + 4 índices de osso num DWORD**
(`SKIN_VERTEX`, `Angelica3D/Header/A3DSkinMesh.h:51-67`) — skinning padrão de GPU; animação = trilhas de
`A3DVECTOR3` e `A3DQUATERNION` por osso (`A3DTrackData.h:303-304`); texturas DDS DXT (o painel já decodifica
DXT1/DXT3, `painel/icones.py`; o WebGL desenha DXT direto pela extensão `WEBGL_compressed_texture_s3tc`).
Tudo tem leitor **e** escritor no fonte (§1).

**O que pesa:** o núcleo de modelo/animação tem ~11 mil linhas de C++ (`A3DSkin` 1.003, `A3DSkinMesh`
1.234, `A3DRigidMesh` 411, `A3DMorphSkinMesh` 364, `A3DSkeleton` 1.168, `A3DBone` 1.027, `A3DJoint` 448,
`A3DTrackMan` 610, `A3DSkinModel` 2.722, `A3DSkinModelAct` 917, `A3DClothMesh` 321, `A3DSuppleMesh` 857), mais
`EC_Model.cpp` (6.708, o `.ecm`); os efeitos `.gfx` são outro sistema inteiro (`GfxCommon`, 71 arquivos,
~51 mil linhas). O cliente desenha com o pipeline fixo do Direct3D 9 + shaders `.sdr`; o navegador não
vai sair igual pixel a pixel.

**Viável, em níveis:**

| nível | o que o painel faz | viável? |
| :--- | :--- | :--- |
| N1 visualizar | modelo com texturas, girar/aproximar, trocar a pele adicional, ver o esqueleto e os pontos de encaixe (hooks) | **sim** — WebGL próprio (a CSP só permite `self`: sem three.js de CDN; ou three.js copiado para `static/` com a licença MIT) |
| N2 animar | listar as ações do `.ecm`, tocar/pausar/velocidade, combinar ações | **sim** — skinning na GPU com as trilhas |
| N3 editar o que é dado | trocar/recolorir textura (matiz, brilho), propriedades do material (transparência, dois lados), propriedades do `.ecm` (escala, ações, velocidades, sons, efeitos ligados), cortar/renomear/mudar a velocidade de uma ação, escala de ossos (`SaveBoneScaleInfo`, `EC_Model.cpp:5247`), mover hooks, trocar arma/pele entre modelos | **sim** — são números e referências, com gravação pelo escritor portado |
| N4 importar/exportar padrão | exportar e importar **glTF** (formato aberto, simples) ↔ `.ski`/`.bon`/`.stck` | **sim** — e qualquer programa (Blender, Maya, ferramentas online) passa a servir |
| N5 modelar/animar do zero | mexer em vértices, UV, pintura de pesos, criar animação por quadros-chave | **não vale** — é reescrever o Blender; fica com o glTF do N4 |
| efeitos `.gfx` | ver partículas no modelo | adiável — 51 mil linhas; no N1–N3, o efeito aparece só como marcador no hook |

**Como fica a arquitetura:** os formatos são lidos e gravados **em Rust no servidor** (`pw-recursos`), com
ida e volta byte a byte como os `.data`; o navegador recebe a malha pronta (buffers de vértices, ossos e
quadros) e só desenha e manda as edições. Um caminho de escrita, validação no servidor, registro de
alterações e o canal de teste do launcher para ver no cliente de verdade.

**Ordem sugerida:** N1 → N2 (já serve para conferir o que se publica) → N3 → N4; o N5 não entra.
Encaixa depois do P4 (`elements`), como P6 da §8.2.

### 8.4 Integração com o Blender e dois cenários (análise de 2026-10-14)

**Fatos do cliente que decidem:**

- **Montaria e cavaleiro:** dois pontos de encaixe com nome — `HH_Ride` no esqueleto da montaria e
  `CC_Ride` no do jogador; o cliente prende o jogador com
  `AddChildModel("Rider", false, "HH_Ride", jogador, "CC_Ride")` (`ElementClient/EC_Player.cpp:2770-2777`);
  garupa: `Rider2` / `CC_Ride2` (`:2051-2068`; nomes em `:140-150`). Mover o `HH_Ride` = mudar onde o
  jogador senta.
- **Peles se ligam ao osso pelo nome** (`A3DSkin`, `m_aBoneNames`, `Angelica3D/Source/A3DSkin.cpp:232-238`):
  uma peça nova (armadura, sela, chifre) pode ser uma pele adicional do mesmo esqueleto sem tocar nas animações.
- **Animações se ligam ao osso pelo índice da junta** (`GetBoneTrack(idJoint)`, `A3DSkinModelAct.cpp:379`;
  trilhas gravadas por `id`, `A3DTrackMan.cpp:184-196`): osso novo **no fim** do esqueleto não muda o índice
  dos outros — todas as animações antigas continuam valendo; o osso novo segue o pai até receber trilhas.
- **Monstro no `elements` v156:** `MONSTER_ESSENCE` com 287 campos — `file_model` (o `.ecm`), `level`, `life`,
  defesas, `attack`/`damage_min`/`damage_max`, alcance e velocidades, `aggressive_mode`, `patroll_mode`,
  `id_strategy`/`common_strategy`/`aggro_strategy_*` (comportamento, ligado ao `aipolicy.data`), habilidades por
  faixa de vida (`skill_hp75_*`…) e **85 campos de drop** (`drop_matters_*`, `drop_mines_*`, `probability_drop_num*`).
  O GS já usa esses valores (spec 05) e já interpreta o `aipolicy` (B127). Onde nasce: `npcgen`.

**Como o painel "chama" o Blender.** O navegador não abre programas da máquina. Três peças, nesta ordem:

1. **Conversor único em Rust no servidor** (`pw-recursos`): formato do jogo (`.ecm`/`.smd`/`.ski`/`.bon`/`.stck`)
   ↔ **glTF + um JSON de metadados** (pontos de encaixe, nomes e velocidades das ações, peles adicionais,
   propriedades do `.ecm`). O glTF é o formato que o Blender importa/exporta nativamente.
2. **Extensão do Blender "PW Universal"** (Python, a instalar no Blender): entra com a conta GM, lista os modelos
   da linha, **baixa** (o servidor converte e manda glTF + metadados; a extensão cria os encaixes como
   *Empties* com o nome `HH_Ride` etc. e as ações como animações nomeadas) e **envia** de volta (exporta glTF +
   metadados; o servidor converte, valida e cria a alteração em rascunho). A extensão não sabe nada do formato do
   jogo — um caminho de escrita só, no servidor.
3. **Atalho "Editar no Blender"** no painel: um endereço `pwuniversal://editar/<linha>/<modelo>` que o
   **launcher** (já instalado na máquina) registra no Windows; ele abre o Blender com a extensão e o modelo.
   Opcional — sem ele, abre-se o modelo pela própria extensão.

Testar: o rascunho vai ao **canal de teste** do launcher e o GM vê no cliente de verdade.

**Cenário 1 — editar uma montaria (partes novas, ossos novos, ponto de montar): viável.**

| passo | onde | situação |
| :--- | :--- | :--- |
| abrir a montaria | painel → "Editar no Blender" | depende do conversor e da extensão |
| modelar a parte nova | Blender | livre |
| prender a parte | Blender: pesar nos ossos existentes **ou** criar ossos novos | ossos novos entram **no fim**; o conversor recusa reordenar/apagar ossos que têm trilha |
| animar a parte nova | Blender: chaves só nos ossos novos, nas ações existentes | o conversor grava trilhas novas nos `.stck`; as antigas ficam iguais |
| ponto de montar | Blender: mover/rotacionar o *Empty* `HH_Ride` (e `HH_Ride2` para a garupa) | gravado no esqueleto (`A3DSkeletonHook`) |
| enviar e testar | extensão → painel (rascunho) → launcher no canal de teste | — |
| item da montaria | painel, editor de `elements` | só se o caminho do modelo mudar |

Cuidados: as ações chamadas pelo cliente para montaria têm nomes fixos (a levantar no fonte antes de fazer);
limites de ossos por malha e de vértices do Direct3D 9 (a levantar); textura sai em DDS (DXT) pelo conversor.

**Cenário 2 — monstro do zero (dados + modelo, armadura e animações fáceis): viável, em duas metades.**

- **Dados (painel):** "Novo monstro a partir de…" copia um `MONSTER_ESSENCE` existente e abre a ficha com os
  campos agrupados (nível, vida, dano, defesas, velocidades, agressividade, patrulha, estratégia, habilidades por
  vida, drops); `id` novo; vale no servidor e no cliente (mesmo `elements`). Comportamento além das estratégias
  existentes = editor do `aipolicy` (outro arquivo, a fazer). Onde nasce = editor de `npcgen` com a vista do mapa.
  Depende do P4 (`elements`) e do P5 (`npcgen`).
- **Modelo (Blender):** o caminho fácil é **partir de um esqueleto existente** (humanoide, quadrúpede, voador…):
  a extensão oferece "Novo modelo sobre o esqueleto de…", que traz o esqueleto e **todas as animações dele** —
  modela-se e texturiza-se a malha, pesa-se nos ossos, e as animações vêm prontas. **Armadura = pele adicional**
  (presa pelo nome do osso): a extensão marca a malha como "pele adicional" e ela vira um `.ski` à parte no
  `.ecm`; dá para trocar de armadura sem mexer no resto. Esqueleto **novo** também funciona, mas aí o artista
  anima cada ação que o cliente toca num monstro (lista de nomes a levantar no fonte).
- **Itens novos de drop** precisam de ícone: o atlas `iconlist_ivtr*.dds` e o `.txt` de nomes (§3, `surfaces`)
  têm de ganhar a célula — o conversor de recursos faz isso na publicação.

| | dados | modelo |
| :--- | :--- | :--- |
| ferramenta | painel (editor de `elements`, `npcgen`, depois `aipolicy`) | Blender + extensão |
| vale no servidor | sim, após reiniciar o realm | — (o servidor não lê modelo) |
| vai ao cliente | sim (`elements`) | sim (arquivos do modelo) |
| risco | campos que o GS ainda não usa (spec 05 lista o que falta) | limites do D3D9; nomes de ação exigidos |

**Ordem (revê o P6 da §8.2):** P6a visualizador N1/N2 no painel (conferir o que se publica) → P6b conversor
jogo ↔ glTF + metadados (com ida e volta byte a byte no lado do jogo) → P6c extensão do Blender (baixar/enviar,
encaixes, ações, pele adicional, "novo sobre o esqueleto de…") → P6d atalho `pwuniversal://` pelo launcher.
O editor do `aipolicy` entra depois do P5. O N3 (edições simples) fica no painel; o N5 (modelar) é o Blender.

> **B231:** linhas de cliente renomeadas para o número da versão (`1.5.5`, `1.2.6`); o Estúdio está em `ESTUDIO_DO_PAINEL.md`.

### 8.5 1.2.6 confirmado em jogo (Murillo, 2026-10-14)

Cópia sem pacotes `F:\Games\perfectworld_126 sepfile\element` (15 pacotes extraídos, 0 falhas): sem `sepfile` o
cliente para no pacote ausente (esperado); **com `sepfile` funciona, com bom desempenho**. Logs limpos: o
`AF.log` só lista os pacotes ausentes, o `EC.log` termina com o fechamento normal da sessão e não há dump; o
`CECWorld::LoadWorld: File operation error (line: 544)` da cena de login aparece igual no 1.2.6 com pacotes. O
1.2.6 dispensou a linha de grupo na `serverlist.txt` (a versão é anterior à janela de grupos). **As duas linhas
(`1.5.5` e `1.2.6`) rodam soltas com `sepfile`.**

### 8.6 Pacote de instalação: formato e carga no servidor

**Formato recomendado: um pacote por pasta, em `.tar.zst`** (zstd nível 19 com janela longa), mais um
manifesto. Comparação:

| | zip (deflate) | **`.tar.zst` por pasta** |
| :--- | :--- | :--- |
| compressão | arquivo a arquivo (como o `.pck`, zlib) | sólida: muitos arquivos pequenos e parecidos comprimem juntos |
| descompactar | rápido | mais rápido (zstd) |
| extrair enquanto baixa | difícil (o índice fica no fim do zip) | **sim**: o launcher extrai em fluxo, sem guardar o pacote — não precisa do dobro de disco nem de "apagar o zip" |
| retomar download | `Range` no arquivo inteiro | `Range` por pacote; pacotes independentes (`models`, `litmodels`, `gfx`…) baixam em paralelo e se reparam sozinhos |
| nome em bytes GBK | sem a marca UTF-8 do zip | cabeçalho GNU com nome longo em bytes (não usar o cabeçalho PAX, que é UTF-8) |

O zip continua possível com a mesma regra de nomes, se o Murillo preferir; a diferença é disco e retomada.

**Manifesto** (`manifesto.json`, assinado com Ed25519, §7.2): versão, lista de pacotes `{nome, sha256,
tamanho}` e lista de arquivos `{caminho_gbk_hex, sha256, tamanho, pacote}`. O launcher confere o hash do pacote,
extrai convertendo cada nome com `MultiByteToWideChar(CP_ACP)` (§7.7), confere tamanho/hash dos arquivos e grava a
versão local; o "reparar" usa a lista de arquivos.

**Carga no servidor** (parte do P1, ferramenta `pw-recursos importar`):

1. Fonte: a **instalação original com os `.pck`** de cada versão — `1.5.5`: `F:\PW\1.5.5\1.5.5 BR\Perfect World
   1.5.5 BR\element`; `1.2.6`: `F:\Games\perfectworld_126\element`. **Não** as cópias de teste (os nomes delas
   já foram convertidos para a ACP desta máquina).
2. Lê cada `.pck` pelo índice (nome em bytes GBK, como o `pw-pck-extract`) e cada solto da instalação (nome
   Unicode no disco → bytes GBK; isso corrige de quebra os cursores com nome chinês que hoje falham nos dois
   clientes).
3. Acrescenta o que é nosso: `serverlist.txt` (com linha de grupo no 1.5.5), atalho/`.bat` com `sepfile`, e
   tira o que não serve (`.pck`, `sPCK.exe`, `pwrepack`, logs, `update5.rar`, pastas `*.pck.files`).
4. Grava a **versão 0** no volume do `pw-recursos` (`data/clientes/<versao>/`: objetos por SHA-256 + índice no
   banco), gera os pacotes `.tar.zst` e o manifesto em `data/clientes/<versao>/atualizacao/0/`, e o serviço os
   serve em `atualizacao/<versao>/…` por HTTP.
5. Conferência: extrair o pacote numa pasta limpa e rodar o cliente com `sepfile` (o mesmo teste do §7.5/§8.5).

Tamanhos e tempos (pacote por pasta, total por versão) se medem no P1.

### 8.7 P1 feito (B233, 2026-10-14)

Serviço `pw-recursos` na porta 8100 — descrição atual em `specs/07_RECURSOS_DO_CLIENTE.md`. Decisões do Murillo:
zstd **12**; assinatura **sem passo manual** (a chave nasce no serviço, num volume que só ele monta — não em
`docker/segredos/`, porque o painel monta `../data` inteiro); importação **dentro do Docker** (no servidor externo, o
cliente original será enviado pelo painel, arquivo a arquivo, e importado lá — P3).

| versão | arquivos | dados | pacotes | importar | baixar e conferir (HTTP local) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| 1.2.6 | 61.234 | 4,7 GB | 26, 2,80 GB | 4 min 31 s | 1 min 21 s |
| 1.5.5 | 327.388 | 18,9 GB | 28, 8,48 GB | 12 min 44 s (1ª vez: 50 min) | 4 min 28 s |

Regras que a importação adotou (evidência na spec 07): pacote vale antes do solto (`AFileImage.cpp:94-104`); solto
sem nome GBK fica fora (234 `Padrão (N).ini` no BR); as 3 entradas corrompidas nos `.pck` do BR ficam fora (como na
cópia `sepfile` que funcionou). Tirados também, a pedido do Murillo: os instaladores avulsos da raiz do BR e, no `data` do 1.2.6, as pastas `BCK`, `Data Export`, `Editados`, `tasks`, os `.rar`, `Notas.txt` e o `npcgen.data` (do servidor). A geração dos pacotes passou de 37 para 7 min no 1.5.5 lendo os objetos em paralelo.

### 8.8 P2 e P3 feitos (B234–B235, 2026-10-14)

- **P2 (launcher):** núcleo `crates/pw-atualizador` + app Tauri `tools/pw-launcher`; confirmado em jogo pelo Murillo
  (baixar, Jogar nas duas versões, Verificar). No B235 a tela ficou guiada pela situação de cada versão (pasta por
  versão, janela de pastas do Windows, "Baixar o jogo" / "Já tenho instalado", atualização detectada ao abrir).
- **P3 (governança):** rascunho → itens por caminho (trava: um rascunho por caminho) → publicar N+1 assinado →
  reverter; API administrativa do `pw-recursos` na 8101 (rede interna + segredo) e página "Recursos do cliente" no
  painel. A versão N reaproveita os pacotes da 0; o que mudou vai por objeto.
- **Ainda do P3:** publicar `servidor`/`ambos` direto no realm (o "Ligação com os realms" do §3); canal de teste
  GM no launcher; envio do cliente original pelo painel (servidor externo). Descrição atual: `specs/07` §5–§6.
