# pw-pck-extract

Leitor/extrator dos pacotes `.pck` (Angelica File Package, o formato que o client
Perfect World usa pra `models.pck`, `litmodels.pck`, `configs.pck`, `interfaces.pck`
etc.) — incluindo o esquema de dois arquivos (`arquivo.pck` + `arquivo.pkx`) que o
engine usa quando o conteúdo passa de ~2GB.

## Por que existe

Achado em 2026-09-03 (Contexto A, 1.5.5): o client 1.5.5 crashava na primeira
renderização completa do mundo porque `models.pck`/`litmodels.pck` (que juntos com
seus `.pkx` passam de 2GB) **nunca abriam** — não por estarem corrompidos ou
incompletos, mas por um bug real do client: `AFilePackGame::InnerOpen`
(`AngelicaFile/Source/AFilePackGame.cpp`, ~linha 155-161) guarda a posição do
índice do pacote num `int` de 32 bits **com sinal**. Pra qualquer pacote cujo
tamanho lógico total (`.pck`+`.pkx`) passe de `INT32_MAX` (2.147.483.647 bytes),
esse valor estoura pra negativo, os `seek()` seguintes vão pra posição errada, e o
pacote inteiro falha silenciosamente — nenhum arquivo dele fica acessível,
independente de onde essa entrada específica estivesse fisicamente armazenada.

Esse é um bug do binário do client (fechado, sem como recompilar igual ao que foi
distribuído) — não tem conserto do lado do servidor. A saída viável é ler esses
pacotes por fora (aqui, em Python, sem o limite de 32 bits) e colocar os arquivos
que faltam como **arquivos soltos** no diretório do client — o sistema de arquivos
do client (`AFile`) sempre procura em disco antes do pacote, então um arquivo solto
com o caminho certo é achado primeiro e nunca chega a precisar abrir o `.pck`
quebrado pra aquele arquivo específico.

Ver o cabeçalho de `pw_pck_extract.py` pra o formato completo (structs, constantes
de máscara, tudo achado por leitura de código + inspeção binária, sem chute).

## Uso

```bash
python pw_pck_extract.py list    <pacote.pck>
python pw_pck_extract.py find    <pacote.pck> <pedaço-do-nome>
python pw_pck_extract.py get     <pacote.pck> <caminho-exato-dentro-do-pacote> <arquivo-de-saida>
python pw_pck_extract.py get-matching <pacote.pck> <pedaço-do-nome> <pasta-de-saida>
python pw_pck_extract.py get-all <pacote.pck> <pasta-de-saida>   # cuidado, pode ser muito grande
```

`<pacote.pck>` pode ser passado com ou sem o `.pkx` ao lado — o script acha sozinho
(mesmo nome, extensão `.pkx`) e trata os dois como um pacote lógico só quando o
`.pkx` existir. Funciona igual pra pacotes menores que 2GB sem `.pkx` nenhum.

`get-matching` é o mais útil na prática: extrai TODAS as entradas cujo nome bate
com um pedaço de texto, preservando a estrutura de pastas — pega um modelo inteiro
(`.ecm` + `.bon` + `.ski` + `.smd` + texturas) numa passada só.

## Validado contra

- `configs.pck` (6MB, sem `.pkx`) — 91 arquivos, listagem e extração conferidas
  byte a byte contra o conteúdo real (`configs\angelica.cfg` extraído e comparado).
- `models.pck` + `models.pkx` (client 1.5.5 EN, ~3,2GB lógicos) — índice completo
  lido sem erro, extração de `models\weapons\人物\刀剑\单手单剑\木剑\*` e
  `models\weapons\人物\初始武器\木剑\*` (a "espada de madeira" cujo `.ecm` faltava,
  ver achado acima) confirmada nos dois clients de teste (EN e BR) como arquivo
  solto.

## Detalhe de build drift

As constantes de máscara/guarda (`AFPCK_MASKDWORD` etc.) batem com o source do
client 1.5.5 (`AngelicaFile/Source/AFilePackMan.cpp`). Só o **tamanho e o layout
exato dos structs `FILEHEADER`/`FILEENTRY_INFILE`** tiveram que ser recalibrados
por inspeção binária (`FILEHEADER` real = 272 bytes, não 280; `FILEENTRY_INFILE`
real = 276 bytes com offset de 32 bits, não 288 com offset de 64) — a build
instalada (2575/2569) diverge do source que temos, o mesmo "desvio de build" já
visto várias vezes nesta sessão pra outras constantes (`GAME_VERSION`, etc.). Se
outra versão/build do client der erro de "check byte" ou tamanho de entrada
errado, é sinal de que essas constantes específicas mudaram de novo — dá pra
recalibrar do mesmo jeito (dump hexadecimal perto do offset do safe header,
procurar os guard bytes 0xFDFDFEEE/0xF00DBEEF pra achar os limites reais do
struct).

## Se quiser portar pra Rust

O projeto já tem um precedente de extrator em Rust+Python pra `elements.data`
(`specs/elements_layouts/`, ver `pw_elements_reader.py` e o `crates/pw-data-loader`
correspondente). Esta ferramenta seguiu o mesmo caminho (Python primeiro, pra
prototipar rápido contra um arquivo real de 3GB+); se virar algo usado com
frequência, vale um port pra `tools/` em Rust seguindo o padrão dos irmãos
(`pw-patch-tool`, `pw-pcapdiff`, `pw-rpcgen`).
