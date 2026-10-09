r"""Classifica os tratadores de addon do servidor 1.5.5 pelo tipo de parâmetro e pela família.

Entrada: `addons.json` (id -> tratador, de `extrair_addons.py`) e o fonte
`F:\PW\1.5.5\EvolvedPWServer\cgame\gs\item\item_addon*.cpp|h`.

Para cada tratador resolve macros (`IDMRA`, `IAERA3`, `SET_ADDON_MACRO`, `STONE_*`,
`TEMPORARY_ADDON_MACRO`), `typedef`s e heranças até chegar a um destes:

- `arg_addon<T>` (`item_addon.cpp:50-235`): `T` = POINT, DOUBLE_POINT, PERCENT,
  DOUBLE_PERCENT, SECOND, DOUBLE_SECOND, DOUBLE_FIX_POINT, TRIPLE_POINT;
- `essence_addon` (soma na essência ao gerar, `item_addon.h:404`), com a família tirada do
  `if(datatype != DT_xxx_ESSENCE) return -1` do `GenerateParam` da classe;
- `addon_handler` com `GenerateParam` próprio (classe à parte, conferida à mão).

Marcas: `conjunto`, `refino`, `pedra`, `temporario` (pela macro) e `float` (o `Activate` lê o
argumento como `float`, mesmo com `arg_addon<POINT>`).

Saída: `classificacao.json` = {"tratadores": {tratador: {...}}, "ids": {id: tratador}} e um
resumo no terminal.

Uso:
    python specs/addons_155/classificar_addons.py [pasta gs/item]
"""
import json
import os
import re
import sys

AQUI = os.path.dirname(os.path.abspath(__file__))
TIPOS = {"POINT", "DOUBLE_POINT", "PERCENT", "DOUBLE_PERCENT", "SECOND", "DOUBLE_SECOND",
         "DOUBLE_FIX_POINT", "TRIPLE_POINT"}


def ler_fonte(pasta):
    partes = {}
    for nome in sorted(os.listdir(pasta)):
        if nome.startswith("item_addon") and nome.endswith((".cpp", ".h")):
            t = open(os.path.join(pasta, nome), encoding="latin1").read()
            t = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), t, flags=re.S)
            t = re.sub(r"//[^\n]*", "", t)
            partes[nome] = t
    return partes


def dividir_args(s):
    """Divide 'a<b,c>,d' no nível de topo."""
    saida, nivel, atual = [], 0, ""
    for ch in s:
        if ch in "<(":
            nivel += 1
        elif ch in ">)":
            nivel -= 1
        if ch == "," and nivel == 0:
            saida.append(atual.strip())
            atual = ""
        else:
            atual += ch
    if atual.strip():
        saida.append(atual.strip())
    return saida


def separar(expr):
    expr = re.sub(r"\s+", "", expr)
    m = re.match(r"^([A-Za-z_]\w*)(?:<(.*)>)?$", expr)
    if not m:
        return expr, []
    return m.group(1), dividir_args(m.group(2)) if m.group(2) else []


def indexar(partes):
    typedefs, classes = {}, {}
    for nome, t in partes.items():
        for m in re.finditer(r"\btypedef\s+([^;]+?)\s+(\w+)\s*;", t, flags=re.S):
            typedefs[m.group(2)] = (re.sub(r"\s+", "", m.group(1)), nome, t[:m.start()].count("\n") + 1)
        for m in re.finditer(r"(template\s*<([^>]*)>\s*)?class\s+(\w+)\s*(?::\s*public\s+([^\{]+?))?\s*\{", t):
            ini = m.end()
            fim = t.find("\n};", ini)
            corpo = t[ini:fim if fim > 0 else ini + 4000]
            params = [p.strip().split()[-1] for p in m.group(2).split(",")] if m.group(2) else []
            base = re.sub(r"\s+", "", m.group(4)) if m.group(4) else ""
            classes.setdefault(m.group(3), (params, base, corpo, nome, t[:m.start()].count("\n") + 1))
    return typedefs, classes


MACROS = [
    (r"^IDMRA\((.*)\)$", lambda a: f"item_decoration_magic_resistance_addon<{a}>", None),
    (r"^IAERA\((.*)\)$", lambda a: f"item_armor_enhance_resistance_addon<{a}>", None),
    (r"^IAERA2\((.*)\)$", lambda a: f"item_armor_enhance_resistance_addon_2<{a}>", None),
    (r"^IAERA3\((.*)\)$", lambda a: f"item_armor_enhance_resistance_addon_3<{a}>", None),
    (r"^STONE_MAGIC_RES_ADDON\((.*)\)$", lambda a: "EPSA_addon<int,X,POINT>", "pedra"),
    (r"^STONE_MAGIC_DMG_ADDON\((.*)\)$", lambda a: "EPSA_addon<int,X,POINT>", "pedra"),
    (r"^SET_ADDON_MACRO\((.*)\)$", lambda a: dividir_args(a)[-1], "conjunto"),
    (r"^TEMPORARY_ADDON_MACRO\((.*)\)$", lambda a: a, "temporario"),
]

FAMILIA_DT = {"DT_WEAPON_ESSENCE": "arma", "DT_ARMOR_ESSENCE": "armadura", "DT_DECORATION_ESSENCE": "acessorio"}


def resolver(expr, typedefs, classes, marcas, contexto=None, prof=0):
    """Devolve (tipo, familia_fonte, onde, classe): `classe` é a classe de essência ou própria
    que tem o `GenerateParam` (None para `arg_addon<T>`)."""
    if prof > 20:
        return ("?", None, "recursão", None)
    expr = re.sub(r"\s+", "", expr)
    for padrao, f, marca in MACROS:
        m = re.match(padrao, expr)
        if m:
            if marca:
                marcas.add(marca)
            return resolver(f(m.group(1)), typedefs, classes, marcas, contexto, prof + 1)
    if contexto and expr in contexto:
        return resolver(contexto[expr], typedefs, classes, marcas, None, prof + 1)
    if expr in TIPOS:
        return (expr, None, "arg_addon", None)
    nome, args = separar(expr)
    if nome.startswith("refine_addon_template"):
        marcas.add("refino")
    if nome == "arg_addon":
        return resolver(args[0], typedefs, classes, marcas, contexto, prof + 1)
    if nome in typedefs and not args:
        return resolver(typedefs[nome][0], typedefs, classes, marcas, contexto, prof + 1)
    if nome in classes:
        params, base, corpo, arq, linha = classes[nome]
        ctx = dict(zip(params, args)) if params else {}
        if contexto:
            ctx = {k: contexto.get(v, v) for k, v in ctx.items()}
        onde = f"{arq}:{linha}"
        if base in ("essence_addon",):
            dts = sorted({FAMILIA_DT[d] for d in re.findall(r"DT_\w+_ESSENCE", corpo) if d in FAMILIA_DT})
            return ("ESSENCIA", "/".join(dts) or None, onde, nome)
        if base in ("addon_handler", ""):
            return ("PROPRIO", None, onde, nome)
        if base == "BASE_HANDLER" or base == "BASE_ADDON" or base == "BASE_ADDON1":
            alvo = ctx.get(base)
            return resolver(alvo, typedefs, classes, marcas, None, prof + 1)
        # O `Activate` que lê o argumento como `float` (`enhance_speed_addon_point`,
        # `item_addon.cpp:356-388`): o valor não é um inteiro somado direto.
        ativa = corpo.find("Activate")
        if ativa >= 0 and "(float*)&" in corpo[ativa:].replace(" ", ""):
            marcas.add("float")
        r = resolver(base, typedefs, classes, marcas, ctx, prof + 1)
        return (r[0], r[1], onde if r[2] == "arg_addon" else r[2], r[3])
    return ("?", None, f"não achado: {nome}", None)


def main():
    pasta = sys.argv[1] if len(sys.argv) > 1 else r"F:\PW\1.5.5\EvolvedPWServer\cgame\gs\item"
    partes = ler_fonte(pasta)
    typedefs, classes = indexar(partes)
    ids = json.load(open(os.path.join(AQUI, "addons.json"), encoding="utf-8"))["tratadores"]
    saida = {}
    for trat in sorted(set(ids.values())):
        marcas = set()
        tipo, fam, onde, classe = resolver(trat, typedefs, classes, marcas)
        saida[trat] = {"tipo": tipo, "classe": classe, "familia_no_fonte": fam, "onde": onde, "marcas": sorted(marcas),
                       "ids": sum(1 for v in ids.values() if v == trat)}
    json.dump({"fonte": "EvolvedPWServer/cgame/gs/item/item_addon*.cpp", "tratadores": saida},
              open(os.path.join(AQUI, "classificacao.json"), "w", encoding="utf-8"),
              ensure_ascii=False, indent=1, sort_keys=True)
    from collections import Counter
    c = Counter()
    for t, v in saida.items():
        c[(v["tipo"], v["familia_no_fonte"], tuple(v["marcas"]))] += v["ids"]
    for k, n in sorted(c.items(), key=lambda x: -x[1]):
        print(n, *k)


if __name__ == "__main__":
    main()
