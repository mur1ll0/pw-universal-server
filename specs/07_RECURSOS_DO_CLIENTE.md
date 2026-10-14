# 07 — Recursos do cliente (`pw-recursos`)

> Como o servidor guarda o cliente de cada versão e o entrega ao launcher. Plano e decisões:
> `docs/admin/RECURSOS_VERSIONAMENTO_E_ATUALIZADOR.md` (§3, §7.2, §7.7, §8, §8.6). Estado: **P1
> (versão 0 + pacotes + HTTP) implementado no B233**; launcher (P2), governança no painel (P3) e
> editores (P4+) faltam.

## 1. Serviço

| | |
| :--- | :--- |
| crate | `crates/pw-recursos` (bin + lib) |
| contêiner | `pw-recursos` (`docker/Dockerfile.recursos`: Rust + Python 3 + `tools/pw-pck-extract`) |
| porta | **8100**, só leitura: `GET /atualizacao/<versao>/<resto>` (com `Range`, `ServeFile` do tower-http) e `GET /saude` |
| volume de dados | `data/clientes` → `/dados` (bind) |
| chave | volume nomeado `pw_recursos_chave` → `/chave/assinatura_ed25519.key`, **montado só aqui** (o painel monta `../data` inteiro) |
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
   `gt_setup.exe`, `gt_updater.exe`, `dxwebsetup.exe`, `unitywebplayerdevelopment.exe`); em `data\`, as pastas
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

## 5. Testes

`cargo test -p pw-recursos` — 16 de unidade (inclui a limpeza de objetos) (tar GNU com nome longo e 2º byte 0x5C, GBK, exclusões,
linha de grupo, `sepfile`, assinatura, fluxo do extrator, rotas) e `tests/versao_zero.rs` (com
`TEST_DATABASE_URL`): instalação de mentira → importar → banco `test` → conferir por pasta e por
HTTP com Range (206) → manifesto adulterado recusado.

## 6. Falta

- P2 launcher (extrair em fluxo, reparar, atualizar por arquivo, canal de teste GM).
- P3 governança no painel; envio do cliente original pelo painel para importar no servidor externo
  (pasta enviada arquivo a arquivo, retomável — decisão B233) e alterações/versões > 0.
- Troca de chave (`chaves.json` assinado pela anterior, §7.2).
