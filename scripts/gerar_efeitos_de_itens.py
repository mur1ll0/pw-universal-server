"""Gera a tabela de texto dos efeitos de item da dica do painel (B192) a partir do cliente 1.5.5.

Lê `CECIvtrEquip::AddOneAddOnPropDesc` (`ElementClient/EC_IvtrEquip.cpp:971-2610`) e, para cada
`case` do `switch (byPropType)`, guarda as chamadas `AddDescText` do ramo **não local** — o do item
que o jogador tem (`Prop.bLocal = false`, `SetItemInfo`, `EC_IvtrEquip.cpp:243`): os
`if (bLocal) {A} else {B}` viram `B`, os `if (!bLocal) {A}` viram `A`, e as macros
`ADD_RANGE_VALUE_DESC_*` (`:980-1028`) viram o seu `value2`.

Cada parte é `[frase ITEMDESC_* | null, literal | null, expressão | null, quebra]`, na ordem do
cliente; `quebra` é o `bRet` (termina a linha). Expressões: `p0`, `p1`, `-p0`, `-p1`, `p0/2`,
`-p0*0.05` (float), `f(p0)` (os bits do int como float), `vp(pN)` (`VisualizeFloatPercent`,
`EC_IvtrItem.h:370`). Os casos de refino (200–212) não escrevem linha (só acumulam) e saem vazios.
`seguro` é falso quando alguma frase passa do índice 112 do `item_desc.txt` BR (onde o arquivo
diverge do cabeçalho do fonte; B189) ou quando o caso não se reduz às partes acima.

Uso: python scripts/gerar_efeitos_de_itens.py
"""
import json
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent
FONTE = pathlib.Path(r"F:\PW\1.5.5\EvolvedPWClient\ElementClient\EC_IvtrEquip.cpp")
INDICES = REPO / "web-admin" / "backend" / "painel" / "item_desc_indices.json"
SAIDA = REPO / "web-admin" / "backend" / "painel" / "efeitos_do_cliente.json"
ULTIMO_SEGURO = 112

EXPRESSOES = {
    "p0": "p0", "(p0)": "p0", "p1": "p1", "-p0": "-p0", "-p1": "-p1", "p0 / 2": "p0/2",
    "-p0 * 0.05f": "-p0*0.05", "*(float*)&p0": "f(p0)",
    "VisualizeFloatPercent(p0)": "vp(p0)", "-VisualizeFloatPercent(p0)": "-vp(p0)",
    "-VisualizeFloatPercent(p1)": "-vp(p1)",
}
# `value2` de cada macro (`EC_IvtrEquip.cpp:1021-1027`).
MACROS = {"NORMAL": "p0", "FLOAT": "f(p0)", "PERCENT": "p0", "MINUS_PERCENT_1": "-p0",
          "MINUS_PERCENT_2": "-vp(p0)", "HALF": "p0/2"}


def bloco(t, i):
    n = 0
    for j in range(i, len(t)):
        if t[j] == "{":
            n += 1
        elif t[j] == "}":
            n -= 1
            if n == 0:
                return j + 1
    return len(t)


def nao_local(t):
    while True:
        m = re.search(r"if\s*\(\s*(!?)\s*bLocal\s*\)\s*", t)
        if not m:
            return t
        neg, i = m.group(1) == "!", m.end()
        if t[i] == "{":
            fim = bloco(t, i)
            a, depois = t[i + 1:fim - 1], fim
        else:
            fim = t.index(";", i) + 1
            a, depois = t[i:fim], fim
        b, resto = "", depois
        mm = re.match(r"\s*else\s*", t[depois:])
        if mm:
            k = depois + mm.end()
            if t[k] == "{":
                f2 = bloco(t, k)
                b, resto = t[k + 1:f2 - 1], f2
            else:
                f2 = t.index(";", k) + 1
                b, resto = t[k:f2], f2
        t = t[:m.start()] + (a if neg else b) + t[resto:]


def partes_do_caso(t):
    """Lista de partes, ou None se o caso tem algo que a tabela não representa."""
    partes = []
    for m in re.finditer(r"AddDescText\s*\(\s*color\s*,\s*(true|false)\s*,\s*(.*?)\)\s*;"
                         r"|ADD_RANGE_VALUE_DESC_(ID|STR)_(\w+?)\s*\(\s*(ITEMDESC_\w+)\s*\)", t, re.S):
        if m.group(1):
            quebra = m.group(1) == "true"
            args = [a.strip() for a in re.split(r",(?![^(]*\))", m.group(2))]
            texto, valor = args[0], (args[1] if len(args) > 1 else None)
            if len(args) > 2:
                return None
            if valor is not None and valor not in EXPRESSOES:
                return None
            expr = EXPRESSOES.get(valor) if valor else None
            f = re.fullmatch(r"pDescTab->GetWideString\s*\(\s*\(?\s*(ITEMDESC_\w+)\s*\)?\s*\)", texto)
            l = re.fullmatch(r'_AL\s*\(\s*"(.*)"\s*\)', texto)
            if f:
                partes.append([f.group(1), None, expr, quebra])
            elif l:
                partes.append([None, l.group(1), expr, quebra])
            else:
                return None
        else:
            tipo, sufixo, frase = m.group(3), m.group(4), m.group(5)
            if tipo == "STR" and sufixo == "NORMAL":
                # `Format("%s %s", frase, "%+d")` e depois `p0` (`:1003-1018`).
                partes += [[frase, None, None, False], [None, " %+d", "p0", True]]
            elif tipo == "ID" and sufixo in MACROS:
                partes.append([frase, None, MACROS[sufixo], True])
            else:
                return None
    return partes


def main():
    linhas = FONTE.read_text(encoding="latin-1").splitlines()
    corpo = "\n".join(linhas[1029:2609])  # o `switch` começa na linha 1030
    corpo = re.sub(r"//[^\n]*", "", corpo)
    indices = json.loads(INDICES.read_text(encoding="utf-8"))
    tabela = {}
    for caso in re.split(r"\n\s*case\s+", corpo)[1:]:
        m = re.match(r"(\d+)\s*:", caso)
        if not m:
            continue
        t = nao_local(caso[m.end():]).split("break;")[0]
        partes = partes_do_caso(t)
        frases = [p[0] for p in partes or [] if p[0]]
        seguro = partes is not None and all(indices.get(f, 10**6) <= ULTIMO_SEGURO for f in frases)
        tabela[m.group(1)] = {"partes": partes or [], "seguro": seguro}
    SAIDA.write_text(json.dumps(tabela, ensure_ascii=False, indent=0), encoding="utf-8")
    seguros = sum(1 for v in tabela.values() if v["seguro"])
    print(f"{len(tabela)} tipos; {seguros} seguros; não seguros: "
          f"{sorted((int(k) for k, v in tabela.items() if not v['seguro']))}")


if __name__ == "__main__":
    main()
