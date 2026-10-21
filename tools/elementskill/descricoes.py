"""B315: os argumentos da descrição (`GetIntroduction`) — do fonte, conferidos palavra a palavra com o gabarito.

    python descricoes.py <pasta dos stubs skillNNN.h> <gabarito.jsonl> <saida.json>

Cada stub do cliente faz `_snwprintf(buffer, length, format, a1, a2, …)`. As expressões `a1…` são do nível (`L`) e,
em 7 habilidades, da proficiência. Aqui elas são avaliadas com a semântica do C (o tipo de cada subexpressão:
`int`, ou `double` se houver literal real ou operando real; divisão inteira entre `int`; conversões `(int)`/`(float)`)
e cada argumento vira as palavras de 32 bits que o `_snwprintf` empilharia (`int` = 1, `double` = 2). Para cada
habilidade, a saída diz se as palavras do fonte batem com as do binário (`intro_bruta` do gabarito) em todos os níveis.
"""
import json
import re
import struct
import sys
from pathlib import Path

PADRAO = re.compile(
    r"int GetIntroduction \(Skill \* skill, wchar_t \* buffer, int length, wchar_t \* format\) const\s*\{(.*?)\n        \}",
    re.S,
)


def argumentos(corpo):
    """As expressões depois de `format` na chamada `_snwprintf (buffer, length, format, …)`."""
    m = re.search(r"_snwprintf\s*\(\s*buffer\s*,\s*length\s*,\s*format\s*(.*)\)\s*;", corpo, re.S)
    if not m:
        return None
    resto = m.group(1).strip()
    if not resto:
        return []
    if not resto.startswith(","):
        return None
    resto = resto[1:]
    args, nivel, atual = [], 0, ""
    for c in resto:
        if c == "(":
            nivel += 1
        elif c == ")":
            nivel -= 1
        if c == "," and nivel == 0:
            args.append(atual.strip())
            atual = ""
        else:
            atual += c
    if atual.strip():
        args.append(atual.strip())
    return args


# ---- avaliador de C com tipos ----
TOKEN = re.compile(r"\s*(?:(\d+\.\d*(?:[eE][-+]?\d+)?f?|\.\d+f?|\d+[eE][-+]?\d+f?)|(\d+)|(skill\s*->\s*\w+\s*\(\s*\))|(\(\s*(?:int|float|double)\s*\))|(\w+)|(&&|\|\||==|!=|<=|>=|[-+*/%()<>?:!]))")


class Expr:
    def __init__(self, texto, variaveis):
        self.t = []
        pos = 0
        texto = texto.strip()
        while pos < len(texto):
            m = TOKEN.match(texto, pos)
            if not m or m.end() == pos:
                raise ValueError(f"token em {texto[pos:pos + 20]!r}")
            pos = m.end()
            real, inteiro, chamada, cast, nome, op = m.groups()
            if real is not None:
                self.t.append(("num", float(real.rstrip("f")), "d"))
            elif inteiro is not None:
                self.t.append(("num", int(inteiro), "i"))
            elif chamada is not None:
                f = re.sub(r"\s", "", chamada)[len("skill->"):-2]
                if f not in variaveis:
                    raise ValueError(f"variável {f}")
                v, tipo = variaveis[f]
                self.t.append(("num", v, tipo))
            elif cast is not None:
                self.t.append(("cast", re.sub(r"[()\s]", "", cast)))
            elif nome is not None:
                raise ValueError(f"nome {nome}")
            else:
                self.t.append(("op", op))
        self.i = 0

    def ver(self):
        return self.t[self.i] if self.i < len(self.t) else None

    def comer(self, op=None):
        x = self.t[self.i]
        if op and x != ("op", op):
            raise ValueError(f"esperava {op}")
        self.i += 1
        return x

    def valor(self):
        v = self.ternario()
        if self.i != len(self.t):
            raise ValueError("sobrou")
        return v

    def ternario(self):
        c = self.binario(0)
        if self.ver() == ("op", "?"):
            self.comer("?")
            a = self.ternario()
            self.comer(":")
            b = self.ternario()
            tipo = "d" if "d" in (a[1], b[1]) else "i"
            r = a if c[0] else b
            return (float(r[0]) if tipo == "d" else r[0], tipo)
        return c

    PRIO = [("||",), ("&&",), ("==", "!="), ("<", ">", "<=", ">="), ("+", "-"), ("*", "/", "%")]

    def binario(self, n):
        if n == len(self.PRIO):
            return self.unario()
        a = self.binario(n + 1)
        while self.ver() and self.ver()[0] == "op" and self.ver()[1] in self.PRIO[n]:
            op = self.comer()[1]
            b = self.binario(n + 1)
            a = self.aplicar(op, a, b)
        return a

    @staticmethod
    def aplicar(op, a, b):
        tipo = "d" if "d" in (a[1], b[1]) else "i"
        x, y = (float(a[0]), float(b[0])) if tipo == "d" else (a[0], b[0])
        if op in ("==", "!=", "<", ">", "<=", ">=", "&&", "||"):
            r = {"==": x == y, "!=": x != y, "<": x < y, ">": x > y, "<=": x <= y, ">=": x >= y,
                 "&&": bool(x) and bool(y), "||": bool(x) or bool(y)}[op]
            return (int(r), "i")
        if op == "+":
            return (x + y, tipo)
        if op == "-":
            return (x - y, tipo)
        if op == "*":
            return (x * y, tipo)
        if op == "/":
            if tipo == "d":
                return (x / y, "d")
            q = abs(x) // abs(y)
            return (q if (x >= 0) == (y >= 0) else -q, "i")
        if op == "%":
            return (int(x - y * int(x / y)), "i")
        raise ValueError(op)

    def unario(self):
        x = self.ver()
        if x == ("op", "-"):
            self.comer()
            v = self.unario()
            return (-v[0], v[1])
        if x == ("op", "+"):
            self.comer()
            return self.unario()
        if x == ("op", "!"):
            self.comer()
            v = self.unario()
            return (int(not v[0]), "i")
        if x and x[0] == "cast":
            self.comer()
            v = self.unario()
            if x[1] == "int":
                return (int(v[0]), "i")
            if x[1] == "float":
                return (float(struct.unpack("<f", struct.pack("<f", float(v[0])))[0]), "d")
            return (float(v[0]), "d")
        if x == ("op", "("):
            self.comer()
            v = self.ternario()
            self.comer(")")
            return v
        return self.comer()[1:]


def palavras(valores):
    """As palavras de 32 bits que o `_snwprintf` recebe (`double` = 2, little-endian)."""
    w = []
    for v, tipo in valores:
        if tipo == "d":
            lo, hi = struct.unpack("<II", struct.pack("<d", float(v)))
            w += [lo, hi]
        else:
            w.append(int(v) & 0xFFFFFFFF)
    return w


def main(pasta, gabarito, destino):
    brutas = {}
    for linha in open(gabarito, encoding="utf-8"):
        x = json.loads(linha)
        if "intro_bruta" in x and x.get("intro_bruta"):
            for chave, ab in (("intro_bruta", 0), ("intro_bruta_500", 500)):
                t = bytes.fromhex(x[chave]).decode("utf-16-le")
                brutas[(x["id"], x["nivel"], ab)] = [int(p, 16) for p in t.split("|")]
    maxability = {}
    resultado, conta = {}, {"batem": 0, "diferem": 0, "sem_leitura": 0}
    niveis = {}
    for (i, n, ab) in brutas:
        niveis.setdefault(i, set()).add(n)
    for f in sorted(Path(pasta).glob("skill*.h")):
        m = re.fullmatch(r"skill(\d+)\.h", f.name)
        if not m:
            continue
        i = int(m.group(1))
        if i not in niveis:
            continue
        t = f.read_text(encoding="latin-1")
        c = PADRAO.search(t)
        args = argumentos(c.group(1)) if c else None
        if args is None:
            resultado[str(i)] = {"args": None}
            conta["sem_leitura"] += 1
            continue
        ok, primeira_falha, tipos = True, None, None
        for n in sorted(niveis[i]):
            for ab in (0, 500):
                # A proficiência máxima do nível vem do gabarito (GetMaxAbility), quando a descrição a usa.
                variaveis = {"GetLevel": (n, "i"), "GetAbility": (ab, "i"), "GetMaxability": (maxability.get((i, n), 0), "i")}
                try:
                    vals = [Expr(a, variaveis).valor() for a in args]
                except (ValueError, ZeroDivisionError, IndexError) as e:
                    ok, primeira_falha = False, f"nível {n}: {e}"
                    break
                tipos = "".join(t for _, t in vals)
                w = palavras(vals)
                b = brutas[(i, n, ab)]
                if b[: len(w)] != w:
                    ok = False
                    primeira_falha = primeira_falha or f"nível {n} prof {ab}: fonte {[hex(x) for x in w]} binário {[hex(x) for x in b[:len(w)]]}"
            if not ok:
                break
        resultado[str(i)] = {"args": args, "tipos": tipos, "bate": ok, **({"falha": primeira_falha} if not ok else {})}
        conta["batem" if ok else "diferem"] += 1
    json.dump({"conta": conta, "habilidades": resultado}, open(destino, "w", encoding="utf-8"), ensure_ascii=False, indent=0)
    print(conta)


if __name__ == "__main__":
    main(*sys.argv[1:])
