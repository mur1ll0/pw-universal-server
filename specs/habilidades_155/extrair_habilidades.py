"""Extrai dos stubs de habilidade do servidor 1.5.5 os números que o pw-gs usa.

Fonte: `F:\\PW\\1.5.5\\EvolvedPWServer\\cskill\\skills\\skillNNN.h` — código gerado
(`cskill/gen/src/Cpp/SkillStub.java`), um arquivo por habilidade, idêntico ao do cliente
(`EvolvedPWClient/ElementSkill`) nos casos conferidos. É o que o servidor original compila:
`SkillStub::Learn`/`LearnCondition` (`cskill/skill/skill.cpp:14-93`) e a conjuração
(`skillwrapper.cpp:242`, `skill.cpp:797`, `playerwrapper.cpp:170`) consultam estas funções.

Saída: `habilidades.json`, uma entrada por habilidade, com os valores **por nível** (1 a
max_level) de tudo o que pôde ser avaliado. Uma expressão que depende de algo além do nível
(o jogador, o alvo) ou que o gerador não conseguiu reconstruir (os ~300 stubs com
`///TODO fix`, que vieram de descompilação com a tabela zerada) sai como `null` — o servidor
trata `null` como desconhecido, nunca como zero.

Uso:
    python specs/habilidades_155/extrair_habilidades.py F:/PW/1.5.5/EvolvedPWServer/cskill/skills
"""
import json
import os
import re
import sys

NIVEL = re.compile(r"(?:skill\s*->\s*GetLevel\s*\(\s*\)|GNET::Skill::GetLevel\s*\(\s*skill\s*\))")


def corpo(texto, assinatura):
    """O corpo `{ ... }` da primeira função cuja declaração contém `assinatura`."""
    i = texto.find(assinatura)
    if i < 0:
        return None
    j = texto.find("{", i)
    prof = 0
    for k in range(j, len(texto)):
        if texto[k] == "{":
            prof += 1
        elif texto[k] == "}":
            prof -= 1
            if prof == 0:
                return texto[j + 1:k]
    return None


def avaliar(expr, nivel):
    """Uma expressão C++ só de nível e números, em Python. `None` se depender de outra coisa."""
    e = NIVEL.sub(f"({nivel})", expr)
    e = re.sub(r"\(\s*float\s*\)", "", e)
    e = re.sub(r"\(\s*int\s*\)\s*\(", "int(", e)
    if re.search(r"[A-Za-z_]", e.replace("int(", "")):
        return None
    try:
        return eval(e, {"__builtins__": {}}, {"int": int})
    except Exception:
        return None


def por_nivel(texto, assinatura, max_level, tipo):
    c = corpo(texto, assinatura)
    if c is None:
        return None
    if "TODO fix" in c:
        return None
    arr = re.search(r"static\s+\w+\s+array\s*\[\s*(\d*)\s*\]\s*=\s*\{([^}]*)\}\s*;\s*return\s+array\s*\[\s*(.+?)\s*\]\s*;", c, re.S)
    valores = []
    for n in range(1, max_level + 1):
        if arr:
            itens = [x.strip() for x in arr.group(2).split(",") if x.strip()]
            # `{ 0 }` num `array[10]` é inicialização agregada: o resto vale zero.
            if arr.group(1):
                itens += ["0"] * (int(arr.group(1)) - len(itens))
            idx = avaliar(arr.group(3), n)
            if idx is None or not (0 <= int(idx) < len(itens)):
                return None
            v = avaliar(itens[int(idx)], n)
        else:
            m = re.search(r"return\s+(.+?)\s*;", c, re.S)
            if not m:
                return None
            v = avaliar(m.group(1), n)
        if v is None:
            return None
        valores.append(tipo(v))
    return valores


def estados(texto, max_level):
    """`GetTime` de cada `StateN` na ordem de `statestub.push_back` — o 1º é a conjuração."""
    ordem = re.findall(r"statestub\.push_back\s*\(\s*new\s+(State\d+)\s*\(\s*\)\s*\)", texto)
    saida = []
    for nome in ordem:
        bloco_i = texto.find(f"class {nome}:")
        if bloco_i < 0:
            bloco_i = texto.find(f"class {nome} :")
        if bloco_i < 0:
            saida.append(None)
            continue
        trecho = texto[bloco_i:]
        saida.append(por_nivel(trecho, "int GetTime", max_level, int))
    return saida


def escalar(texto, campo, tipo=int):
    m = re.search(rf"\b{campo}\s*=\s*([-\d.]+)\s*;", texto)
    return tipo(float(m.group(1))) if m else None


def extrair(caminho):
    texto = open(caminho, encoding="gbk", errors="replace").read()
    m = re.search(r"SkillStub\s*\(\s*(\d+)\s*\)", texto)
    if not m:
        return None
    sid = int(m.group(1))
    max_level = escalar(texto, "max_level") or 1
    return {
        "id": sid,
        "cls": escalar(texto, "cls"),
        "max_level": max_level,
        "type": escalar(texto, "type"),
        "rank": escalar(texto, "rank"),
        "attr": escalar(texto, "attr"),
        "apcost": escalar(texto, "apcost"),
        "apgain": escalar(texto, "apgain"),
        "arrowcost": escalar(texto, "arrowcost"),
        "commoncooldown": escalar(texto, "commoncooldown"),
        "commoncooldowntime": escalar(texto, "commoncooldowntime"),
        "pre_skills": [
            [int(a), int(b)]
            for a, b in re.findall(r"pre_skills\.push_back\s*\(\s*std::pair\s*<\s*ID\s*,\s*int\s*>\s*\(\s*(\d+)\s*,\s*(\d+)\s*\)\s*\)", texto)
        ],
        "restrict_weapons": [int(x) for x in re.findall(r"restrict_weapons\.push_back\s*\(\s*(\d+)\s*\)", texto)],
        "mp": por_nivel(texto, "float GetMpcost", max_level, float),
        "execucao_ms": por_nivel(texto, "int GetExecutetime", max_level, int),
        "recarga_ms": por_nivel(texto, "int GetCoolingtime", max_level, int),
        "nivel_exigido": por_nivel(texto, "int GetRequiredLevel", max_level, int),
        "sp_exigido": por_nivel(texto, "int GetRequiredSp", max_level, int),
        "dinheiro_exigido": por_nivel(texto, "int GetRequiredMoney", max_level, int),
        "estados_ms": estados(texto, max_level),
    }


def main():
    pasta = sys.argv[1] if len(sys.argv) > 1 else r"F:\PW\1.5.5\EvolvedPWServer\cskill\skills"
    saida = {}
    for nome in sorted(os.listdir(pasta)):
        if not re.fullmatch(r"skill\d+\.h", nome):
            continue
        h = extrair(os.path.join(pasta, nome))
        if h:
            saida[str(h["id"])] = h
    destino = os.path.join(os.path.dirname(os.path.abspath(__file__)), "habilidades.json")
    with open(destino, "w", encoding="utf-8") as f:
        json.dump({"fonte": "EvolvedPWServer/cskill/skills", "habilidades": saida}, f, ensure_ascii=False, separators=(",", ":"))
    total = len(saida)
    def conta(campo):
        return sum(1 for h in saida.values() if h[campo] is not None)
    print(f"{total} habilidades -> {destino}")
    for campo in ("mp", "execucao_ms", "recarga_ms", "nivel_exigido", "sp_exigido", "dinheiro_exigido"):
        print(f"  {campo}: {conta(campo)} avaliadas")
    print(f"  estados: {sum(1 for h in saida.values() if h['estados_ms'] and all(e is not None for e in h['estados_ms']))} completos")


if __name__ == "__main__":
    main()
