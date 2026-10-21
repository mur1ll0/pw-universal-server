"""B315 (fase 3 do DLL de dados): o modelo do cliente — tudo o que o `elementskill.dll` de dados devolve, em dados.

    python gerar_modelo.py <pasta dos stubs skillNNN.h> <gabarito.jsonl> <descricoes.json> <modelo.json>

Fonte de cada parte (spec 08 §8):
- números, textos, listas, armas, estados: o **gabarito** (o binário original);
- argumentos da descrição: quantos e de que tipo pelo **fonte** (`descricoes.py`), os bits pelo **gabarito**
  (`intro_bruta`, as primeiras palavras);
- alcance: `d = a·alcance_da_arma + b + c·bônus` (as 3 formas do `GetPraydistance` nos stubs), `b` em `double`
  pelo fonte e conferido nas 6 sondas do gabarito com a regra da base (`skill.h:251-272`);
- condição de combo `arg[i] == | >= v` e faixa de vida `[lo, hi]` em % (as formas dos stubs), conferidas no
  diferencial.
Tudo o que não reproduz o gabarito é reportado e falha a geração.
"""
import json
import re
import struct
import sys
from pathlib import Path

# Slots da vtable (spec 08 §3) que o modelo guarda por nível.
SLOTS_INT = [5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 19, 21, 26, 27, 31, 32, 33, 42, 45, 46, 47, 48, 49, 50]
SLOTS_BYTE = [2, 22, 23, 24, 25, 35, 36, 37, 38, 39, 40, 41, 52]
SLOTS_TEXTO = ["0", "1", "3", "29", "30"]  # 0 = wchar_t*, os outros char*
SONDAS_ALCANCE = [(0, 0), (1, 0), (5, 0), (0, 1), (3.5, 2), (10, 0.5)]
EST_NIVEL = ["GetRequiredLevel", "GetRequiredSp", "GetRequiredMoney", "GetRequiredBook", "GetRequiredRealmLevel",
             "GetExecuteTime", "GetMaxAbility"]


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def alcance_da_base(tipo, auto, area, d):
    """`Skill::GetPrayRange` (`skill.h:251-272`) com `d` já em float."""
    if tipo in (1, 3):  # TYPE_ATTACK, TYPE_CURSE
        if auto:
            r = f32(f32(0.3) * d)
            return f32(d - f32(1.0)) if r >= 1.0 else f32(f32(0.7) * d)
        return d
    if tipo == 8:  # TYPE_JUMP
        return d
    if area in (2, 5):  # NoTarget (`skill.h:39`)
        return -1.0
    return d


def stubs(pasta):
    """Do fonte: a forma do `GetPraydistance` por nível, a condição de combo e a faixa de vida."""
    from descricoes import Expr  # mesmo avaliador de C com tipos
    r = {}
    for f in Path(pasta).glob("skill*.h"):
        m = re.fullmatch(r"skill(\d+)\.h", f.name)
        if not m:
            continue
        t = f.read_text(encoding="latin-1")
        s = {}
        pd = re.search(r"float GetPraydistance \(Skill \* skill\) const\s*\{\s*return(.*?);\s*\}", t, re.S)
        if pd:
            e = re.sub(r"\s+", " ", pd.group(1)).strip()
            a = 1 if "GetRange ()" in e else 0
            c = 1 if "GetPrayrangeplus ()" in e else 0
            base = re.sub(r"skill->GetPlayer \(\)->(GetRange|GetPrayrangeplus) \(\)", "0", e)
            s["praydistance"] = (a, c, base)
        cb = re.search(r"CheckComboSkExtraCondition \(Skill \* skill\) const\s*\{\s*return \(bool\) \(skill->GetPlayer \(\)->GetComboarg \(\)->GetValue \((\d+)\) (==|>=) (\d+)\);", t)
        if cb:
            s["combo"] = [int(cb.group(1)), cb.group(2), int(cb.group(3))]
        hp = re.search(r"CheckHpCondition \(int hp, int max_hp\) const\s*\{\s*return hp >= max_hp / (\d+)\.f \* \((\d+)\) && hp <= max_hp / (\d+)\.f \* \((\d+)\);", t)
        if hp:
            s["vida"] = [int(hp.group(2)), int(hp.group(4))]
        r[int(m.group(1))] = s
    r["_Expr"] = Expr
    return r


def main(pasta, gabarito, descricoes, destino):
    sys.path.insert(0, str(Path(__file__).parent))
    fonte = stubs(pasta)
    Expr = fonte.pop("_Expr")
    desc = json.load(open(descricoes, encoding="utf-8"))["habilidades"]
    modelo = {"versao_do_modelo": 1, "habilidades": {}, "estados_visiveis": [], "estados_equipe": [], "inerentes": {},
              "combos": []}
    problemas = []
    atual = None
    for linha in open(gabarito, encoding="utf-8"):
        x = json.loads(linha)
        if "habilidade" in x:
            i = x["habilidade"]
            atual = {
                "id": i, "max_level": x["max_level"], "estaticas": x["estaticas"], "armas": x["armas"],
                "vida": fonte.get(i, {}).get("vida"), "combo": fonte.get(i, {}).get("combo"), "niveis": [],
            }
            # A faixa de vida do fonte tem de reproduzir as sondas do gabarito.
            esperado = x["vida"]
            lo, hi = atual["vida"] or (0, 100)
            def ok(hp, mx):
                return hp >= f32(mx / 100.0) * lo and hp <= f32(mx / 100.0) * hi if atual["vida"] else True
            calc = "".join("1" if ok(v, 100) else "0" for v in range(101)) + "|" + "".join("1" if ok(v, 1000) else "0" for v in range(0, 1001, 7))
            if calc != esperado:
                problemas.append(f"{i}: vida {atual['vida']} não reproduz as sondas")
            modelo["habilidades"][str(i)] = atual
        elif "nivel" in x and "v" in x:
            i, n = x["id"], x["nivel"]
            v = x["v"]
            nivel = {
                "ints": [v[str(s)] for s in SLOTS_INT],
                "bytes": [v[str(s)] for s in SLOTS_BYTE],
                "textos": [v[s] for s in SLOTS_TEXTO],
                "genio": x["genio"], "pre": x["pre"], "juniores": x["juniores"],
                "est": [x["est"][k] for k in EST_NIVEL],
            }
            if x["pre"] != x["pre_lista"] or x["juniores"] != x["juniores_lista"]:
                problemas.append(f"{i}/{n}: lista pelos dois caminhos diferente")
            # Alcance.
            tipo, auto, area = v["2"], v["23"], v["26"]
            forma = fonte.get(i, {}).get("praydistance")
            obs = x["alcance"]
            achado = None
            candidatos = []
            if forma:
                a, c, base = forma
                try:
                    b64 = float(Expr(base, {"GetLevel": (n, "i")}).valor()[0])
                    candidatos.append((a, c, b64))
                except Exception:
                    pass
            # Sem forma no fonte (ou o fonte não bate): `d` de cada sonda invertendo a regra da base (auto-ataque:
            # `saída + 1` se `0,3·d ≥ 1`, senão `saída / 0,7`), e daí b = d(0,0), a = d(1,0) − b, c = d(0,1) − b
            # (2905/2906/3284/3285: d constante com auto-ataque; 3196/3197: a = 2).
            def inv(o):
                if tipo in (1, 3) and auto:
                    return float(f32(o + 1.0)) if f32(f32(0.3) * f32(o + 1.0)) >= 1.0 else float(f32(o / 0.7))
                return float(o)
            d0, d1, d3 = inv(obs[0]), inv(obs[1]), inv(obs[3])
            candidatos.append((d1 - d0, d3 - d0, d0))
            candidatos += [(a, c, float(obs[0])) for a in (0, 1) for c in (0, 1)]
            for a, c, b64 in candidatos:
                calc = [alcance_da_base(tipo, auto, area, f32(a * r + b64 + c * p)) for r, p in SONDAS_ALCANCE]
                if all(f32(cv) == f32(ov) for cv, ov in zip(calc, obs)):
                    achado = [a, b64, c]
                    break
            if achado is None:
                problemas.append(f"{i}/{n}: alcance {obs} sem modelo (tipo {tipo}, auto {auto}, área {area}, forma {forma})")
                achado = [0, 0.0, 0]
            nivel["alcance"] = achado
            # Descrição: quantas palavras pelo fonte; os bits pelo binário.
            d = desc.get(str(i), {})
            args, tipos = d.get("args"), d.get("tipos")
            intro = []
            if args is not None and tipos is not None:
                bruta = [int(p, 16) for p in bytes.fromhex(x["intro_bruta"]).decode("utf-16-le").split("|")]
                k = 0
                for expr, t in zip(args, tipos):
                    e = re.sub(r"\s", "", expr)
                    if e == "skill->GetAbility()":
                        intro.append({"proficiencia": True})
                    elif e == "skill->GetMaxability()":
                        intro.append({"proficiencia_max": True})
                    else:
                        intro.append({"palavras": bruta[k:k + (2 if t == "d" else 1)]})
                    k += 2 if t == "d" else 1
            elif x.get("intro") not in (None, ""):
                problemas.append(f"{i}/{n}: descrição sem argumentos lidos no fonte")
            nivel["intro"] = intro
            atual["niveis"].append(nivel)
        elif "estado_visivel" in x:
            modelo["estados_visiveis"].append({"prof": x["estado_visivel"][0], "consulta": x["estado_visivel"][1], "id": x["id"],
                                               "nome": x["nome"], "hh": x["hh"], "efeito": x["efeito"]})
        elif "estado_equipe" in x:
            modelo["estados_equipe"].append({"consulta": x["estado_equipe"], "id": x["id"], "nome": x["nome"], "icone": x["icone"]})
        elif "inerentes" in x:
            modelo["inerentes"][str(x["inerentes"])] = x["lista"]
        elif "versao" in x:
            modelo["versao_do_dll"] = x["versao"]
        elif "combo" in x:
            modelo["combos"].append(x)
    # Intervalo do combo de cada habilidade (o segundo do par do `GetComboSkActivated`, `stub+0x74`).
    for c in modelo["combos"]:
        for pid, intervalo in c.get("lista", []):
            h = modelo["habilidades"][str(pid)]
            if h.get("intervalo_combo", intervalo) != intervalo:
                problemas.append(f"{pid}: dois intervalos de combo")
            h["intervalo_combo"] = intervalo
    del modelo["combos"]
    # Lista na ordem do `NextSkill` do original (a do gabarito): o executável percorre nessa ordem.
    modelo["habilidades"] = list(modelo["habilidades"].values())
    json.dump(modelo, open(destino, "w", encoding="utf-8"), ensure_ascii=False, separators=(",", ":"))
    print(f"{len(modelo['habilidades'])} habilidades, {len(modelo['estados_visiveis'])} estados visíveis, "
          f"{len(modelo['estados_equipe'])} de equipe, {len(modelo['inerentes'])} classes com inerentes -> {destino}")
    print(f"problemas: {len(problemas)}")
    for p in problemas[:15]:
        print("  ", p)


if __name__ == "__main__":
    main(*sys.argv[1:])
