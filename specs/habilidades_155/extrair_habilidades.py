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


ALCANCE_DA_ARMA = re.compile(r"skill\s*->\s*GetPlayer\s*\(\s*\)\s*->\s*GetRange\s*\(\s*\)")
BONUS_DE_ALCANCE = re.compile(r"skill\s*->\s*GetPlayer\s*\(\s*\)\s*->\s*GetPrayrangeplus\s*\(\s*\)")


def alcance(texto, max_level):
    """`GetPraydistance` por nível, separando o alcance de ataque do jogador.

    `GetRange()` é `GetExtendProp().attack_range` (`playerwrapper.h:95`) — o alcance da arma
    com o corpo. Sai como `{"arma": k, "fixo": [...]}`: alcance = `k × attack_range + fixo`.
    `GetPrayrangeplus()` é bônus de talento/equipamento e vale 0 aqui (sem esses sistemas).
    """
    c = corpo(texto, "float GetPraydistance")
    if c is None or "TODO fix" in c:
        return None
    m = re.search(r"return\s+(.+?)\s*;", c, re.S)
    if not m:
        return None
    expr = BONUS_DE_ALCANCE.sub("0", m.group(1))
    k = 1 if ALCANCE_DA_ARMA.search(expr) else 0
    fixo = []
    for n in range(1, max_level + 1):
        v = avaliar(ALCANCE_DA_ARMA.sub("0", expr), n)
        if v is None:
            return None
        fixo.append(float(v))
    if k:
        # Confere que o alcance entra somado, com coeficiente 1.
        for n in range(1, max_level + 1):
            um = avaliar(ALCANCE_DA_ARMA.sub("1", expr), n)
            if um is None or abs(float(um) - fixo[n - 1] - 1.0) > 1e-6:
                return None
    return {"arma": k, "fixo": fixo}


SETTER_DE_DANO = re.compile(
    r"skill\s*->\s*Set(Damage|Golddamage|Wooddamage|Waterdamage|Firedamage|Earthdamage)\s*\(\s*"
    r"(?:([\d.]+)\s*\*\s*)?skill\s*->\s*Get(Attack|Magicattack)\s*\(\s*\)\s*\)\s*;"
)
CARGA = re.compile(r"skill\s*->\s*GetCharging\s*\(\s*\)")


def argumento(corpo_calc, setter):
    """A expressão passada a `skill->SetX(...)`, com parênteses balanceados."""
    m = re.search(rf"skill\s*->\s*{setter}\s*\(", corpo_calc)
    if not m:
        return None
    i, prof = m.end(), 1
    for k in range(i, len(corpo_calc)):
        if corpo_calc[k] == "(":
            prof += 1
        elif corpo_calc[k] == ")":
            prof -= 1
            if prof == 0:
                return corpo_calc[i:k]
    return None


def dano(texto, max_level, estados_ms):
    """A conta de dano da habilidade: o estado com `SetDamage`/`SetXdamage(k * GetAttack())`.

    O servidor faz `GeneratePhysicDamage((int)(ratio*100), (int)plus)` ou
    `GenerateMaigicDamage2(...)` (`skill.cpp:897-898`, `skill.h:634`, `actobject.h:1422-1469`):
    dano sorteado da arma e base × (100 + bônus do atributo + ratio×100)/100 + plus, vezes o
    fator `k`. Em habilidade de carga, `GetCharging()` é o tempo carregado; aqui sai com a
    carga **cheia** (o tempo do 1º estado) e `carga: true` — o mundo escala pelo tempo real.
    """
    ordem = re.findall(r"statestub\.push_back\s*\(\s*new\s+(State\d+)\s*\(\s*\)\s*\)", texto)
    for idx, nome in enumerate(ordem):
        i = texto.find(f"class {nome}:")
        if i < 0:
            i = texto.find(f"class {nome} :")
        if i < 0:
            continue
        calc = corpo(texto[i:], "void Calculate")
        if not calc or "TODO fix" in calc:
            continue
        m = SETTER_DE_DANO.search(calc)
        if not m:
            continue
        elemento, fator, base = m.group(1), float(m.group(2) or 1.0), m.group(3)
        carga = False
        ratio, plus = [], []
        for n in range(1, max_level + 1):
            valores = []
            for setter in ("SetRatio", "SetPlus"):
                expr = argumento(calc, setter)
                if expr is None:
                    valores.append(0.0)
                    continue
                if CARGA.search(expr):
                    carga = True
                    cheia = (estados_ms or [None])[0]
                    if not cheia or cheia[n - 1] is None:
                        return None
                    expr = CARGA.sub(f"({cheia[n - 1]})", expr)
                v = avaliar(expr, n)
                if v is None:
                    return None
                valores.append(float(v))
            ratio.append(valores[0])
            plus.append(valores[1])
        return {
            "estado": idx,
            "base": "fisico" if base == "Attack" else "magico",
            "elemento": elemento,
            "fator": fator,
            "ratio": ratio,
            "plus": plus,
            "carga": carga,
        }
    return None


def distancia(texto, assinatura, max_level):
    """`k × GetRange() + fixo[nível]` de uma função de distância (`GetEffectdistance`...).

    Mesmo formato de [`alcance`]; `None` quando a expressão depende de outra coisa.
    """
    c = corpo(texto, assinatura)
    if c is None or "TODO fix" in c:
        return None
    m = re.search(r"return\s+(.+?)\s*;", c, re.S)
    if not m:
        return None
    expr = BONUS_DE_ALCANCE.sub("0", m.group(1))
    k = 1 if ALCANCE_DA_ARMA.search(expr) else 0
    fixo = []
    for n in range(1, max_level + 1):
        v = avaliar(ALCANCE_DA_ARMA.sub("0", expr), n)
        if v is None:
            return None
        fixo.append(float(v))
    if k:
        for n in range(1, max_level + 1):
            um = avaliar(ALCANCE_DA_ARMA.sub("1", expr), n)
            if um is None or abs(float(um) - fixo[n - 1] - 1.0) > 1e-6:
                return None
    return {"arma": k, "fixo": fixo}


# Getters que viram variáveis da expressão; o servidor (`efeitos::Expr`) conhece estas.
_VARIAVEIS = [
    (re.compile(r"skill\s*->\s*GetPlayer\s*\(\s*\)\s*->\s*Get(\w+)\s*\(\s*\)"), r"P_\1"),
    (re.compile(r"skill\s*->\s*GetTarget\s*\(\s*\)\s*->\s*Get(\w+)\s*\(\s*\)"), r"A_\1"),
    (re.compile(r"skill\s*->\s*GetVictim\s*\(\s*\)\s*->\s*Get(\w+)\s*\(\s*\)"), r"V_\1"),
    (re.compile(r"skill\s*->\s*GetLevel\s*\(\s*\)"), "L"),
    (re.compile(r"skill\s*->\s*Get(\w+)\s*\(\s*\)"), r"S_\1"),
]
_SETTER = re.compile(r"^skill\s*->\s*(?:(GetVictim|GetPlayer)\s*\(\s*\)\s*->\s*)?Set(\w+)\s*\((.*)\)$", re.S)


def expressao(e):
    """Uma expressão C++ de stub no formato que `efeitos::Expr` lê: números, `+ - * /`,
    comparações, `?:`, `&& ||`, parênteses, `INT(...)` e variáveis `L`, `P_X`, `V_X`,
    `A_X`, `S_X`. `None` quando sobra algo que não é isso."""
    e = " ".join(e.split())
    e = re.sub(r"\(\s*float\s*\)", "", e)
    e = re.sub(r"\(\s*int\s*\)\s*\(", "INT(", e)
    for rx, sub in _VARIAVEIS:
        e = rx.sub(sub, e)
    resto = re.sub(r"\b(INT|L|[PAVS]_\w+)\b", "", e)
    resto = re.sub(r"\d+\.?\d*(e[-+]?\d+)?f?", "", resto)
    if re.search(r"[A-Za-z_]", resto) or re.search(r"[^\s()+\-*/<>=!?:&|]", resto):
        return None
    return e


def roteiro(texto, nome):
    """O corpo de `StateAttack`/`BlessMe` como lista de `[quem, setter, expressão]`.

    `quem`: `V` (a vítima), `P` (quem conjura), `S` (a própria habilidade). Corpo com
    controle de fluxo (`if`, `for`, `switch`) ou expressão que não se deixa ler sai `None`
    — o servidor não aplica o que não leu.
    """
    c = corpo(texto, f"bool {nome} (Skill * skill) const")
    if c is None:
        c = corpo(texto, f"bool {nome}(Skill * skill) const")
    if c is None:
        return None
    if "TODO fix" in c or re.search(r"\b(if|for|while|switch)\b", c):
        return None
    passos = []
    for s in c.split(";"):
        s = " ".join(s.split())
        # Comentário de linha antes do comando (ex.: "// Added in 1.5.5 ...").
        if "//" in s:
            s = re.sub(r"//.*?(?=skill\s*->|$)", "", s).strip()
        if not s or s.startswith("return") or s == "{" or s == "}":
            continue
        m = _SETTER.match(s.strip("{} "))
        if not m:
            return None
        quem = {"GetVictim": "V", "GetPlayer": "P", None: "S"}[m.group(1)]
        e = expressao(m.group(3))
        if e is None:
            return None
        passos.append([quem, m.group(2), e])
    return passos


def escalar(texto, campo, tipo=int):
    """O valor de `campo = <valor>;` no stub.

    Aceita `true`/`false` além de número: os stubs escrevem os dois para o mesmo campo
    (`is_movingcast = 1` em 5 deles, `= true` em 19), e um padrão que só casasse dígito
    deixa 19 habilidades com o campo ausente — foi o que aconteceu no B78 e tirou do
    Tormentador o conjurar andando das duas habilidades que ele de fato usa (B82).
    """
    m = re.search(rf"\b{campo}\s*=\s*([-\d.]+|true|false)\s*;", texto)
    if not m:
        return None
    bruto = m.group(1)
    if bruto == "true":
        bruto = "1"
    elif bruto == "false":
        bruto = "0"
    return tipo(float(bruto))


def extrair(caminho):
    texto = open(caminho, encoding="gbk", errors="replace").read()
    m = re.search(r"SkillStub\s*\(\s*(\d+)\s*\)", texto)
    if not m:
        return None
    sid = int(m.group(1))
    max_level = escalar(texto, "max_level") or 1
    h = {
        "id": sid,
        "cls": escalar(texto, "cls"),
        "max_level": max_level,
        "type": escalar(texto, "type"),
        "rank": escalar(texto, "rank"),
        "attr": escalar(texto, "attr"),
        "apcost": escalar(texto, "apcost"),
        "apgain": escalar(texto, "apgain"),
        "arrowcost": escalar(texto, "arrowcost"),
        # `is_movingcast` (`cskill/skill/skill.h:382`): a habilidade pode ser conjurada
        # andando — `playercmd.cpp:2066-2088` a manda por `moving_skill` em vez de
        # `session_skill`, e o movimento não a cancela.
        "is_movingcast": escalar(texto, "is_movingcast"),
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
        # O livro que o aprendizado consome (`SkillWrapper::PetLearn`, `skillwrapper.cpp:1558-1564`).
        "item_exigido": por_nivel(texto, "int GetRequiredItem", max_level, int),
        "estados_ms": estados(texto, max_level),
        # `time_type == 3` é conjuração com carga que o jogador solta (`Skill::IsWarmup`,
        # `skill.h:571`): o tempo carregado vira `GetCharging()`.
        "time_type": escalar(texto, "time_type"),
        "alcance": alcance(texto, max_level),
    }
    h["dano"] = dano(texto, max_level, h["estados_ms"])
    # Área, precisão e efeitos (B53) — `PlayerWrapper::SetPerform` (`playerwrapper.cpp:170-420`).
    m = re.search(r"range\.type\s*=\s*(\d+)\s*;", texto)
    h["tipo_de_area"] = int(m.group(1)) if m else None
    h["doenchant"] = bool(re.search(r"doenchant\s*=\s*true", texto))
    h["dobless"] = bool(re.search(r"dobless\s*=\s*true", texto))
    h["raio"] = por_nivel(texto, "float GetRadius", max_level, float)
    h["distancia_de_ataque"] = por_nivel(texto, "float GetAttackdistance", max_level, float)
    h["angulo"] = por_nivel(texto, "float GetAngle", max_level, float)
    h["distancia_de_efeito"] = distancia(texto, "float GetEffectdistance", max_level)
    h["precisao"] = por_nivel(texto, "float GetHitrate", max_level, float)
    h["no_alvo"] = roteiro(texto, "StateAttack")
    h["em_si"] = roteiro(texto, "BlessMe")
    return h


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
    for campo in ("mp", "execucao_ms", "recarga_ms", "nivel_exigido", "sp_exigido", "dinheiro_exigido", "item_exigido"):
        print(f"  {campo}: {conta(campo)} avaliadas")
    print(f"  estados: {sum(1 for h in saida.values() if h['estados_ms'] and all(e is not None for e in h['estados_ms']))} completos")
    for campo in ("tipo_de_area", "raio", "angulo", "distancia_de_efeito", "precisao", "no_alvo", "em_si"):
        print(f"  {campo}: {conta(campo)} lidas")


if __name__ == "__main__":
    main()
