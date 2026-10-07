"""Textos de item do cliente 1.5.5 BR para a dica do painel (B189).

Os mesmos arquivos que o cliente carrega (`CECGame::Init`, `EC_Game.cpp:517-525`;
`LoadItemExtProps`, `:2022-2088`), extraídos do `configs.pck` para `data/textos/` por
`scripts/gerar_textos_de_itens.py`:

- `item_desc.txt` — frases-modelo, na ordem da enumeração `ITEMDESC_*` (`EC_FixedMsg.h:375`),
  indexada por `item_desc_indices.json`;
- `item_ext_desc.txt` — `id "texto"`, a descrição própria do item (`AddExtDescText`);
- `item_color.txt` — `id índice`, a cor do nome (`GetItemNameColorIdx`, `EC_Game.cpp:2389-2410`).

Todos em UTF-16 com BOM; as frases usam `\\r` como quebra e `^RRGGBB` como cor.
"""
import functools
import json
import os
import re
from pathlib import Path

# `l_aNameCols` (`EC_Game.cpp:136-150`): índice 0–9 → cor do nome (ARGB; o painel usa RGB).
CORES_DO_NOME = ["ffffff", "8080ff", "ffdc50", "aa32ff", "ff6000", "ffffff", "b0b0b0", "00ffae", "ff0000", "ffffff"]
_FRASE = re.compile(r'"((?:[^"\\]|\\.)*)"')


def _pasta_padrao():
    aqui = Path(__file__).resolve()
    # `parents[3]` só existe no repositório (no contêiner o arquivo está em /app/painel).
    niveis = [n for n in (1, 3) if n < len(aqui.parents)]
    for candidata in (aqui.parents[n] / "data" / "textos" for n in niveis):
        if candidata.is_dir():
            return candidata
    return aqui.parents[1] / "data" / "textos"


PASTA = Path(os.getenv("TEXTOS_DIR") or _pasta_padrao())


def _ler(nome):
    # `item_desc`/`item_ext_desc` vêm em UTF-16 com BOM; `item_color` é texto simples.
    bruto = (PASTA / nome).read_bytes()
    if bruto[:2] in (bytes([0xFF, 0xFE]), bytes([0xFE, 0xFF])):
        return bruto.decode("utf-16")
    return bruto.decode("latin-1")


def _desfazer_escape(s):
    return s.replace('\\"', '"').replace("\\\\", "\\")


@functools.lru_cache(maxsize=1)
def frases():
    corpo = _ler("item_desc.txt").split("#_begin", 1)[1]
    lista = [_desfazer_escape(m.group(1)) for m in _FRASE.finditer(corpo)]
    indices = json.loads((Path(__file__).parent / "item_desc_indices.json").read_text(encoding="utf-8"))
    # Além do 112 o arquivo BR diverge do cabeçalho: valem só as posições conferidas no binário
    # (B193, `efeitos_no_binario_br.json`); as outras ficam fora em vez de pegar a frase errada.
    binario = Path(__file__).parent / "efeitos_no_binario_br.json"
    conferidas = json.loads(binario.read_text(encoding="utf-8"))["frases"] if binario.exists() else {}
    indices = {nome: i for nome, i in indices.items() if i <= 112}
    indices.update(conferidas)
    return {nome: lista[i] for nome, i in indices.items() if i < len(lista)}


@functools.lru_cache(maxsize=1)
def descricoes():
    corpo = _ler("item_ext_desc.txt").split("#_begin", 1)[1]
    tabela = {}
    for linha in corpo.splitlines():
        m = re.match(r'^\s*(\d+)\s+(".*)$', linha)
        if m:
            f = _FRASE.match(m.group(2))
            if f:
                tabela.setdefault(int(m.group(1)), _desfazer_escape(f.group(1)))
    return tabela


@functools.lru_cache(maxsize=1)
def cores():
    tabela = {}
    for linha in _ler("item_color.txt").splitlines():
        partes = linha.split()
        if len(partes) >= 2 and partes[0].isdigit() and partes[1].isdigit():
            tabela.setdefault(int(partes[0]), int(partes[1]))
    return tabela


@functools.lru_cache(maxsize=1)
def tipos_de_efeito():
    """`item_ext_prop.txt` → {id do efeito: tipo} (`LoadItemExtProps`, `EC_Game.cpp:2022-2060`):
    blocos `tipo { id id … }`; o `AScriptFile` pula os comentários `/* */` e `//`."""
    texto = re.sub(r"/\*.*?\*/", " ", _ler("item_ext_prop.txt"), flags=re.S)
    texto = re.sub(r"//[^\n]*", " ", texto)
    tabela = {}
    for m in re.finditer(r"(\d+)\s*\{([^}]*)\}", texto):
        for idprop in re.findall(r"\d+", m.group(2)):
            tabela.setdefault(int(idprop), int(m.group(1)) & 0xFF)  # colisão: fica o primeiro (`put`)
    return tabela


@functools.lru_cache(maxsize=1)
def efeitos_do_cliente():
    """Texto de cada tipo de efeito, gerado por `scripts/gerar_efeitos_de_itens.py` (B192)."""
    return json.loads((Path(__file__).parent / "efeitos_do_cliente.json").read_text(encoding="utf-8"))


# Nome da classe como o cliente escreve (B190): `CECGameRun::GetProfName` (`EC_GameRun.cpp:3448`)
# lê `FIXMSG_PROF_*` do `fixed_msg.txt`; no `configs.pck` 1.5.5 BR as posições 32–39, 229–230 e
# 282–283 batem com a enumeração (`EC_FixedMsg.h:62-70, 298, 300, 362-363`). O 1.2.6 usa 0–7.
NOMES_DAS_CLASSES = ["Guerreiro", "Mago", "Espirit.", "Feiticeira", "Bárbaro", "Merc.", "Arqueiro",
                     "Sacer.", "Arcano", "Místico", "Retalh.", "Torment."]


def nome_da_classe(classe):
    return NOMES_DAS_CLASSES[classe] if 0 <= classe < len(NOMES_DAS_CLASSES) else f"Classe {classe}"


def nomes_das_classes():
    return NOMES_DAS_CLASSES


def cor_do_nome(tid):
    i = cores().get(tid, 0)
    return CORES_DO_NOME[i if 0 <= i < 10 else 0]


def frase(nome, *args):
    """Frase-modelo `ITEMDESC_*` com os `%d`/`%s`/`%.2f` do cliente preenchidos."""
    modelo = frases().get(nome, "")
    try:
        return modelo % args if args else modelo
    except (TypeError, ValueError):
        return modelo
