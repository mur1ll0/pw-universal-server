"""Roteiros (`StateAttack`/`BlessMe`) do 1.2.6 gerados do `gs` 1.2.6, executando o stub.

O catálogo 1.2.6 herdava do 1.5.5 os roteiros `no_alvo`/`em_si` (lidos do fonte C++ dos stubs
1.5.5). O `gs` 1.2.6 tem só o binário, então aqui cada `SkillNNNStub::StateAttack` (→ `no_alvo`)
e `BlessMe` (→ `em_si`) é **executado** no emulador do extrator (`Emulador`), nível a nível, com
as consultas (`Get*`) respondidas por sondas:

- `Skill::GetLevel` é o nível (`L`); `GetPlayer`/`GetVictim`/`GetTarget` devolvem três ponteiros
  distintos, e o `this` de cada chamada seguinte diz de quem é a consulta ou o setter — `P_`
  (quem conjura), `V_` (a vítima), `A_` (o alvo), `S_` (a habilidade), como no extrator 1.5.5
  (`specs/habilidades_155/extrair_habilidades.py:243-247`);
- qualquer outro `Get*` devolve uma sonda (int em `EAX`, ou `float` na pilha x87 para os que o
  `playerwrapper.h` 1.5.5 declara `float`, linhas 95-267);
- cada `Set*`/efeito é gravado **em ordem**, com o argumento.

A expressão de cada passo sai assim, nesta ordem:
1. a expressão herdada do 1.5.5 no mesmo passo (mesmo setter, alinhado por nome), **se ela
   reproduz os números do `gs` 1.2.6** em todos os níveis e sondas;
2. senão, ajustada: função só do nível (constante, `a·L + b`, ou tabela `L == 1 ? … : …`);
   afim nas consultas de que depende (`c(L) + k(L)·X`), conferida com sondas combinadas;
   `P_Cls`/`V_Cls` por partes (`X == -1 ? a : b`), porque o stub testa monstro (`cls` −1);
3. senão, o passo não é lido — e o roteiro inteiro sai `None` (o servidor não aplica o que não
   leu), a menos que o herdado tenha exatamente a mesma sequência de setters (fica o herdado, e
   o relatório diz).
"""
import difflib
import math
import random
import re
import struct

import unicorn.x86_const as X
from unicorn import UC_HOOK_CODE

PONTEIROS = {"GetPlayer": 0x3000, "GetVictim": 0x3100, "GetTarget": 0x3200}
QUEM = {0x1000: "S", 0x3000: "P", 0x3100: "V", 0x3200: "A"}
# `float Get*` do `PlayerWrapper` 1.5.5 (`cskill/skill/playerwrapper.h:95-267`); os da `Skill`
# consultados nos roteiros são `int` (`skill.h:652-725`).
GETTERS_FLOAT = {"Range", "Skillenhance2", "Rangetotarget", "Prayrangeplus", "Probability", "Ratio",
                 "Value", "Amount", "Incgold", "Incwood", "Incwater", "Incfire", "Incearth"}
METODO = re.compile(r"_ZNK?4GNET(5Skill|13PlayerWrapper)(\d+)(\w+)$")
TOL = 2e-4


# ------------------------------------------------------------------ expressões (igual ao Rust)
def _tokens(s):
    ops = ["&&", "||", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "(", ")", "?", ":", "!"]
    i, t = 0, []
    while i < len(s):
        c = s[i]
        if c.isspace():
            i += 1
        elif c.isdigit() or (c == "." and i + 1 < len(s) and s[i + 1].isdigit()):
            m = re.match(r"\d*\.?\d*(?:[eE][-+]?\d+)?[fF]?", s[i:])
            t.append(float(m.group(0).rstrip("fF")))
            i += len(m.group(0))
        elif c.isalpha() or c == "_":
            m = re.match(r"\w+", s[i:])
            t.append(("n", m.group(0)))
            i += len(m.group(0))
        else:
            for o in ops:
                if s.startswith(o, i):
                    t.append(o)
                    i += len(o)
                    break
            else:
                raise ValueError(s)
    return t


def analisar(expr):
    """A expressão como árvore: número, `("v", nome)`, `(op, a, b)`, `("?", c, a, b)`,
    `("neg", a)`, `("!", a)`, `("INT", a)` — a gramática de `efeitos::expr` (`efeitos.rs:26-230`)."""
    t, i = _tokens(expr), [0]

    def op(o):
        if i[0] < len(t) and t[i[0]] == o:
            i[0] += 1
            return True
        return False

    def ternario():
        c = binario(0)
        if op("?"):
            a = ternario()
            if not op(":"):
                raise ValueError(expr)
            return ("?", c, a, ternario())
        return c

    niveis = [["||"], ["&&"], ["==", "!="], ["<=", ">=", "<", ">"], ["+", "-"], ["*", "/"]]

    def binario(k):
        if k == len(niveis):
            return unario()
        a = binario(k + 1)
        while True:
            for o in niveis[k]:
                if op(o):
                    a = (o, a, binario(k + 1))
                    break
            else:
                return a

    def unario():
        if op("-"):
            return ("neg", unario())
        if op("!"):
            return ("!", unario())
        x = t[i[0]]
        i[0] += 1
        if isinstance(x, float):
            return x
        if x == "(":
            v = ternario()
            op(")")
            return v
        if isinstance(x, tuple):
            if x[1] == "INT":
                op("(")
                v = ternario()
                op(")")
                return ("INT", v)
            return ("v", x[1])
        raise ValueError(expr)

    arv = ternario()
    if i[0] != len(t):
        raise ValueError(expr)
    return arv


def _inteira(no, literais):
    """O subtermo é `int` em C? Variável `int` (as de `GETTERS_FLOAT` não) e literal sem ponto."""
    if isinstance(no, float):
        return no in literais
    if no[0] == "v":
        return no[1] == "L" or no[1].split("_", 1)[1] not in GETTERS_FLOAT
    if no[0] in ("neg", "INT"):
        return no[0] == "INT" or _inteira(no[1], literais)
    if no[0] == "?":
        return _inteira(no[2], literais) and _inteira(no[3], literais)
    if no[0] in ("+", "-", "*", "/"):
        return _inteira(no[1], literais) and _inteira(no[2], literais)
    return True  # comparação e lógica valem 0/1 (int)


def calcular(no, vars_, c=False, literais=frozenset()):
    """Valor da árvore. `c`: divisão entre inteiros trunca, como no stub C++."""
    if isinstance(no, float):
        return no
    k = no[0]
    if k == "v":
        return float(vars_[no[1]])
    if k == "neg":
        return -calcular(no[1], vars_, c, literais)
    if k == "!":
        return float(calcular(no[1], vars_, c, literais) == 0)
    if k == "INT":
        return float(math.trunc(calcular(no[1], vars_, c, literais)))
    if k == "?":
        return calcular(no[2] if calcular(no[1], vars_, c, literais) != 0 else no[3], vars_, c, literais)
    a, b = calcular(no[1], vars_, c, literais), calcular(no[2], vars_, c, literais)
    if k == "/":
        if b == 0:
            return 0.0
        if c and _inteira(no, literais):
            return float(math.trunc(a / b))
        return a / b
    return float({"+": a + b, "-": a - b, "*": a * b, "==": a == b, "!=": a != b, "<": a < b, ">": a > b,
                  "<=": a <= b, ">=": a >= b, "&&": a != 0 and b != 0, "||": a != 0 or b != 0}[k])


def literais_inteiros(expr):
    """Os literais escritos sem ponto (`100`, não `1.0` nem `0.5`) — o tipo C do número."""
    return frozenset(float(x) for x in re.findall(r"(?<![\w.])(\d+)(?![\w.])", expr))


def texto(no, c=False, literais=frozenset()):
    """A árvore de volta ao texto; com `c`, a divisão inteira sai `INT(a / b)`."""
    if isinstance(no, float):
        return _num(no)
    k = no[0]
    if k == "v":
        return no[1]
    if k == "neg":
        return f"-({texto(no[1], c, literais)})"
    if k == "!":
        return f"!({texto(no[1], c, literais)})"
    if k == "INT":
        return f"INT({texto(no[1], c, literais)})"
    if k == "?":
        return f"({texto(no[1], c, literais)} ? {texto(no[2], c, literais)} : {texto(no[3], c, literais)})"
    s = f"({texto(no[1], c, literais)} {k} {texto(no[2], c, literais)})"
    if k == "/" and c and _inteira(no, literais):
        return f"INT{s}"
    return s


def avaliar(expr, vars_, c=False):
    """`efeitos::expr::avaliar` em Python (com `c`, a divisão inteira do C++)."""
    return calcular(analisar(expr), vars_, c, literais_inteiros(expr))


def variaveis(expr):
    return {n for n in re.findall(r"\b([PVAS]_\w+|L)\b", expr) if n != "INT"}


# ------------------------------------------------------------------ execução com sondas
class Roteirista:
    def __init__(self, emu):
        self.emu = emu
        self.metodos = {}
        for s in emu.simbolos:
            m = METODO.match(s.name)
            if m:
                nome = m.group(3)[: int(m.group(2))]
                self.metodos[s["st_value"]] = (m.group(1), nome, s.name.endswith("f"))

    def executar(self, endereco, nivel, sonda):
        """Os passos `(quem, setter, valor)` de `f(this, skill)`, e as consultas feitas."""
        mu, passos, consultas = self.emu.mu, [], set()

        def gancho(uc, ad, _t, _d):
            c = self.metodos.get(ad)
            if not c:
                return
            classe, nome, arg_float = c
            esp = uc.reg_read(X.UC_X86_REG_ESP)
            volta, this = struct.unpack("<II", uc.mem_read(esp, 8))
            quem = QUEM.get(this, "S" if classe == "5Skill" else "?")
            if classe == "5Skill" and nome == "GetLevel":
                uc.reg_write(X.UC_X86_REG_EAX, nivel)
            elif nome in PONTEIROS:
                uc.reg_write(X.UC_X86_REG_EAX, PONTEIROS[nome])
            elif nome.startswith("Get"):
                var = f"{quem}_{nome[3:]}"
                consultas.add(var)
                v = sonda(var)
                if nome[3:] in GETTERS_FLOAT:
                    self.emu._empilhar(float(v))
                else:
                    uc.reg_write(X.UC_X86_REG_EAX, int(v) & 0xFFFFFFFF)
            elif nome.startswith(("Is", "Has", "Can")):
                var = f"{quem}_{nome}"
                consultas.add(var)
                uc.reg_write(X.UC_X86_REG_EAX, int(sonda(var)) & 0xFFFFFFFF)
            else:
                bruto = bytes(uc.mem_read(esp + 8, 4))
                valor = struct.unpack("<f" if arg_float else "<i", bruto)[0]
                passos.append((quem, nome[3:] if nome.startswith("Set") else nome, float(valor)))
                uc.reg_write(X.UC_X86_REG_EAX, 1)
            uc.reg_write(X.UC_X86_REG_ESP, esp + 4)
            uc.reg_write(X.UC_X86_REG_EIP, volta)

        h = mu.hook_add(UC_HOOK_CODE, gancho)
        esp = 0x7000_0000
        for v in (0x1000, 0x2000, 0x7100_0000):
            esp -= 4
            mu.mem_write(esp, struct.pack("<I", v))
        mu.reg_write(X.UC_X86_REG_ESP, esp)
        mu.reg_write(X.UC_X86_REG_EBP, 0)
        mu.reg_write(X.UC_X86_REG_FPSW, 0)
        mu.reg_write(X.UC_X86_REG_FPTAG, 0xFFFF)
        try:
            mu.emu_start(endereco, 0x7100_0000, count=20000)
        finally:
            mu.hook_del(h)
        return passos, consultas


def sonda_base(var, k=0):
    """Valor de sonda por variável e por rodada `k`: distinto entre variáveis e rodadas."""
    if var.endswith("_Cls"):
        return [-1, 0, 3, 4, 1, 5][k % 6]
    if var.endswith("_Rand"):
        return [37, 5, 88, 61, 20, 73][k % 6]
    if var.endswith("_Form"):
        return [0, 1, 0, 65, 0, 1][k % 6]
    h = sum(ord(c) * (i + 1) for i, c in enumerate(var))
    return 101 + (h * 37 + k * 211) % 877


def _num(v):
    v = float(f"{v:.6g}")
    return str(int(v)) if v == int(v) and abs(v) < 1e9 else repr(v)


def expr_de_nivel(vals):
    """Os valores por nível (índice 0 = nível 1) como expressão em `L`."""
    if all(abs(v - vals[0]) <= TOL * max(1, abs(vals[0])) for v in vals):
        return _num(vals[0])
    a = vals[1] - vals[0]
    b = vals[0] - a
    a, b = float(f"{a:.6g}"), float(f"{b:.6g}")
    if all(abs(a * (n + 1) + b - v) <= TOL * max(1, abs(v)) for n, v in enumerate(vals)):
        return f"{_num(a)} * L + {_num(b)}" if b else f"{_num(a)} * L"
    if len(vals) >= 4:
        # `a·L² + b·L + c` pelos três primeiros níveis, conferido nos outros.
        y1, y2, y3 = vals[:3]
        a2 = float(f"{(y3 - 2 * y2 + y1) / 2:.6g}")
        b2 = float(f"{(y2 - y1) - 3 * a2:.6g}")
        c2 = float(f"{y1 - a2 - b2:.6g}")
        if all(abs(a2 * (n + 1) ** 2 + b2 * (n + 1) + c2 - v) <= TOL * max(1, abs(v)) for n, v in enumerate(vals)):
            return f"{_num(a2)} * L * L + {_num(b2)} * L + {_num(c2)}"
    corpo = _num(vals[-1])
    for n in range(len(vals) - 2, -1, -1):
        corpo = f"L == {n + 1} ? {_num(vals[n])} : ({corpo})"
    return corpo


def _perto(a, b):
    return abs(a - b) <= TOL * max(1.0, abs(a), abs(b))


class Gerador:
    def __init__(self, emu):
        self.r = Roteirista(emu)

    def _rodar(self, f, n, sonda):
        return [self.r.executar(f, nv, sonda) for nv in range(1, n + 1)]

    def gerar(self, f, n, herdado):
        """(roteiro, situação). Situação: 'igual', 'gerado', 'herdado', 'nao_lido', 'vazio'."""
        if f is None:
            return ([], "vazio") if herdado else ([], "igual")
        rodadas = []  # (k, sonda, [(passos, consultas) por nível])
        for k in range(4):
            s = (lambda k: lambda v: sonda_base(v, k))(k)
            rodadas.append((k, s, self._rodar(f, n, s)))
        nomes = [tuple((q, nm) for q, nm, _ in p) for _, _, rs in rodadas for p, _ in rs]
        consultas = set().union(*(c for _, _, rs in rodadas for _, c in rs))
        seq_h = [(p[0], p[1]) for p in (herdado or [])]
        if len(set(nomes)) != 1:
            return self._fallback(herdado, rodadas, n, "fluxo")
        seq = list(nomes[0])
        if not seq:
            return ([], "igual" if not herdado else "gerado")
        # 1. o herdado inteiro reproduz?
        if herdado and seq == seq_h and self._reproduz(herdado, rodadas, n):
            return herdado, "igual"
        alinh = {}
        sm = difflib.SequenceMatcher(a=[x[1] for x in seq], b=[x[1] for x in seq_h], autojunk=False)
        for bloco in sm.get_matching_blocks():
            for d in range(bloco.size):
                alinh[bloco.a + d] = bloco.b + d
        saida = []
        for i, (quem, nome) in enumerate(seq):
            amostras = [(nv + 1, s, rs[nv][0][i][2]) for _, s, rs in rodadas for nv in range(n)]
            e = None
            if i in alinh:
                cand = herdado[alinh[i]][2]
                # O texto herdado, e o mesmo com a divisão inteira do C++ explícita em `INT(...)`
                # (o `efeitos::expr` divide em ponto flutuante); e os dois truncados, quando o stub
                # guarda o resultado num `int` antes do setter (a 116: 766,59 → `SetValue(766)`).
                cand_c = texto(analisar(cand), True, literais_inteiros(cand))
                for c in (cand, cand_c, f"INT({cand})", f"INT({cand_c})"):
                    if self._expr_reproduz(c, amostras):
                        e = c
                        break
            if e is None:
                e = self._ajustar(f, n, i, consultas, amostras)
            if e is None:
                return self._fallback(herdado, rodadas, n, f"passo {i} {nome}")
            saida.append([quem, nome, e])
        return saida, "gerado"

    def _fallback(self, herdado, rodadas, n, motivo):
        seq = [tuple((q, nm) for q, nm, _ in p) for _, _, rs in rodadas for p, _ in rs]
        seq_h = tuple((p[0], p[1]) for p in (herdado or []))
        if herdado and all(s == seq_h for s in seq):
            return herdado, f"herdado ({motivo})"
        return None, f"nao_lido ({motivo})"

    def _reproduz(self, rot, rodadas, n):
        for _, s, rs in rodadas:
            for nv in range(n):
                passos = rs[nv][0]
                for (q, nm, v), p in zip(passos, rot):
                    if not self._vale(p[2], nv + 1, s, v):
                        return False
        return True

    def _vale(self, expr, nivel, s, v):
        try:
            vars_ = {x: (nivel if x == "L" else sonda_valor(s, x)) for x in variaveis(expr)}
            return _perto(avaliar(expr, vars_), v)
        except (ValueError, KeyError, IndexError, ZeroDivisionError):
            return False

    def _expr_reproduz(self, expr, amostras):
        return all(self._vale(expr, nv, s, v) for nv, s, v in amostras)

    def _valor(self, f, nivel, i, sonda):
        passos, _ = self.r.executar(f, nivel, sonda)
        return passos[i][2] if i < len(passos) else None

    def _ajustar(self, f, n, i, consultas, amostras):
        base = lambda v: sonda_base(v, 1)
        dep = []
        for var in sorted(consultas):
            def outra(v, var=var):
                if v != var:
                    return sonda_base(v, 1)
                # `Cls`/`Form`: −1/0 contra 0/1 — o stub testa monstro e forma.
                return sonda_base(v, 0) if v.endswith(("_Cls", "_Form")) else sonda_base(v, 1) + 7
            for nv in range(1, n + 1):
                a, b = self._valor(f, nv, i, base), self._valor(f, nv, i, outra)
                if a is None or b is None or not _perto(a, b):
                    dep.append(var)
                    break
        if not dep:
            vals = [self._valor(f, nv, i, base) for nv in range(1, n + 1)]
            e = expr_de_nivel(vals)
            return e if self._expr_reproduz(e, amostras) else None
        cls = [v for v in dep if v.endswith(("_Cls", "_Form"))]
        if cls:
            if len(dep) != 1:
                return None
            var = cls[0]
            valores = {-1: None, 0: None} if var.endswith("_Cls") else {0: None, 1: None}
            ramo = {}
            for c in valores:
                s = lambda v, c=c: c if v == var else sonda_base(v, 1)
                ramo[c] = expr_de_nivel([self._valor(f, nv, i, s) for nv in range(1, n + 1)])
            a, b = list(ramo.values())
            chave = list(ramo)[0]
            e = f"{var} == {chave} ? ({a}) : ({b})" if a != b else a
            return e if self._expr_reproduz(e, amostras) else None
        # afim: v = c(L) + Σ k_j(L)·X_j
        termos, c_vals = {}, []
        for nv in range(1, n + 1):
            v0 = self._valor(f, nv, i, base)
            c = v0
            for var in dep:
                s1 = lambda v, var=var: sonda_base(v, 1) + (10 if v == var else 0)
                k = (self._valor(f, nv, i, s1) - v0) / 10
                termos.setdefault(var, []).append(k)
                c -= k * sonda_base(var, 1)
            # resto de ponto flutuante (0,00000610 na 833) é zero
            c_vals.append(0.0 if abs(c) < 1e-3 else c)
        c = expr_de_nivel(c_vals)
        partes = [] if c == "0" else [c if re.fullmatch(r"-?[\d.]+", c) else f"({c})"]
        for var, ks in termos.items():
            k = expr_de_nivel(ks)
            partes.append(var if k == "1" else f"{var} * {k}" if re.fullmatch(r"-?[\d.]+", k) else f"{var} * ({k})")
        e = " + ".join(partes)
        return e if self._expr_reproduz(e, amostras) else None


def sonda_valor(sonda, var):
    return sonda(var)
