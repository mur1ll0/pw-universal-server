"""B317: o diferencial — o gabarito do nosso DLL contra o do original (spec 08 §10).

    python diferencial.py <gabarito_original.jsonl> <gabarito_nosso.jsonl> <modelo.json> <formatos.txt>

Igualdade exata em todo campo, com duas exceções medidas no próprio original (B314):
- `intro_bruta*`: só as palavras que são argumentos (o número delas vem do modelo); o resto é lixo da pilha;
- `intro*` das habilidades cujo formato do `skillstr.txt` consome mais palavras do que os argumentos (`%` sem escape na
  tradução, "100% de" → `% d`): o original mostra o que estiver na pilha — às vezes muda entre rodadas, às vezes é um
  endereço fixo do próprio DLL original (0x1001B001 = 268546049). O nosso mostra 0. A lista sai do formato.
"""
import re
import collections
import json
import sys

CONVERSAO = re.compile(r"%[-+ #0]*(\*|\d+)?(?:\.(\*|\d+))?(hh|h|ll|l|L|I64|I32|I)?([a-zA-Z%])")


def palavras_consumidas(formato):
    """Quantas palavras de 32 bits o `_snwprintf` lê para o formato."""
    n = 0
    for m in CONVERSAO.finditer(formato):
        largura, precisao, tamanho, conv = m.groups()
        if conv == "%":
            continue
        n += (largura == "*") + (precisao == "*")
        n += 2 if conv in "eEfgGaA" or tamanho in ("ll", "I64", "L") else 1
    return n


def main(original, nosso, modelo, formatos):
    m = json.load(open(modelo, encoding="utf-8"))
    nargs = {}
    for h in m["habilidades"]:
        for k, n in enumerate(h["niveis"], 1):
            nargs[(h["id"], k)] = sum(len(a.get("palavras", [0])) for a in n["intro"])
    fmt = {}
    for l in open(formatos, encoding="ascii"):
        p = l.split()
        fmt[int(p[0])] = bytes.fromhex(p[1]).decode("utf-16-le") if len(p) > 1 else ""
    indeterminadas = {i for (i, k), n in nargs.items() if palavras_consumidas(fmt.get(i, "")) > n}
    print(f"habilidades com formato que consome além dos argumentos: {len(indeterminadas)}")
    dif = collections.Counter()
    exemplos = {}
    linhas = 0
    a_linhas = open(original, encoding="utf-8").read().splitlines()
    b_linhas = open(nosso, encoding="utf-8").read().splitlines()
    if len(a_linhas) != len(b_linhas):
        print(f"número de linhas: original {len(a_linhas)}, nosso {len(b_linhas)}")
    for la, lb in zip(a_linhas, b_linhas):
        linhas += 1
        if la == lb:
            continue
        x, y = json.loads(la), json.loads(lb)
        chaves = set(x) | set(y)
        for k in chaves:
            vx, vy = x.get(k), y.get(k)
            if vx == vy:
                continue
            if k.startswith("intro_bruta") and vx and vy:
                n = nargs.get((x.get("id"), x.get("nivel")), 0)
                wx = bytes.fromhex(vx).decode("utf-16-le").split("|")[:n]
                wy = bytes.fromhex(vy).decode("utf-16-le").split("|")[:n]
                if wx == wy:
                    continue
            if k.startswith("intro") and x.get("id") in indeterminadas:
                continue
            if isinstance(vx, dict) and isinstance(vy, dict):
                for s in set(vx) | set(vy):
                    if vx.get(s) != vy.get(s):
                        dif[f"{k}.{s}"] += 1
                        exemplos.setdefault(f"{k}.{s}", (x.get("id", x.get("habilidade")), x.get("nivel"), vx.get(s), vy.get(s)))
                continue
            dif[k] += 1
            exemplos.setdefault(k, (x.get("id", x.get("habilidade")), x.get("nivel"), str(vx)[:80], str(vy)[:80]))
    print(f"linhas comparadas: {linhas}; campos diferentes: {sum(dif.values())}")
    for k, v in dif.most_common(30):
        print(f"  {k}: {v}  ex. {exemplos[k]}")


if __name__ == "__main__":
    main(*sys.argv[1:])
