# 07 — Recursos do cliente (`pw-recursos`)

> Como o servidor guarda o cliente de cada versão e o entrega ao launcher. Plano e decisões:
> `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (§3, §7.2, §7.7, §8, §8.6). Estado: **P1
> (versão 0 + pacotes + HTTP) no B233, confirmado em jogo nas duas versões; P2 (núcleo do launcher
> `pw-atualizador` + app Tauri `tools/pw-launcher`) no B234, confirmado em jogo; usabilidade do launcher e P3
> (rascunho, publicar e reverter pelo painel) no B235**; editores (P4+) e o envio do cliente original pelo painel faltam.

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

**Ainda não feito:** arquivos de destino `servidor`/`ambos` (elements, tasks, npcgen) não são copiados para
o `data/<realm>/config` nem marcam reinício (§3 "Ligação com os realms") — hoje isso é manual.

## 7. Testes

`cargo test -p pw-recursos` — 16 de unidade (inclui a limpeza de objetos) (tar GNU com nome longo e 2º byte 0x5C, GBK, exclusões,
linha de grupo, `sepfile`, assinatura, fluxo do extrator, rotas) e `tests/versao_zero.rs` (com
`TEST_DATABASE_URL`): instalação de mentira → importar → banco `test` → conferir por pasta e por
HTTP com Range (206) → verificar/reparar → atualização da versão 0 para uma 1 montada à mão → manifesto
adulterado recusado; adotar uma pasta sem estado. `tests/governanca.rs`: API com segredo (401 sem),
rascunho idempotente, trava por caminho (409), publicar v1 com mínima, o launcher atualiza uma pasta e
instala outra do zero na v1 (2 por objeto, 2 pulados), reverter → v2 igual à 0. `cargo test -p
pw-atualizador`: tar, nomes. Painel: `web-admin/backend/tests/teste_recursos.py` (5: sessão, CSRF, autor,
segredo, limite de envio, download) — rodar no contêiner do painel (ver histórico B235).

## 8. Falta

- Launcher: canal de teste para GM (instalar um rascunho antes de publicar), autoatualização do próprio
  launcher, instalador `.msi` (CLI do Tauri).
- Publicar arquivo de destino `servidor`/`ambos` direto no realm (troca atômica + `aguarda_reinicio`).
- Envio do cliente original pelo painel para importar no servidor externo (pasta enviada arquivo a
  arquivo, retomável — decisão B233).
- Pacotes novos de tempos em tempos (hoje a versão N reaproveita os da 0 e o resto vai por objeto).
- Troca de chave (`chaves.json` assinado pela anterior, §7.2).
