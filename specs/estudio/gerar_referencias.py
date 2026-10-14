"""Gera `specs/estudio/referencias.json`: que campo `int32` de cada tabela do `elements.data` cita o
id de outro registro, e em que espaço (Estúdio S2, B238).

Duas evidências, as duas exigidas:

1. **Fonte:** o membro do `ExpTypes.h` do cliente 1.5.5 (achatado como os layouts fazem:
   `nome_i_membro`, `i` a partir de 1) tem nome de id (`id_*`, `*_id`, `id`) ou comentário que diz o que
   cita (ID, 道具/物品 = item, 怪物 = monstro, 模板 = modelo…) — e não é habilidade (技能), missão (任务),
   mapa/mundo, facção, ícone etc., que não são registros do `elements`.
2. **Dados:** nos `elements.data` dos realms (1.2.6 e 1.5.5), ao menos 95% dos valores não nulos do campo
   existem como id no espaço de destino (`espacos_de_id.json`).

O espaço sai do comentário/nome (附加属性/addon → ADDON, 配方/recipe → RECIPE, 对话/talk → TALK) ou, se
nada disser, é o de maior cobertura. Campo que só passa numa das evidências fica de fora e vai para
`duvidosos` com o motivo — para revisar à mão, nunca para entrar por palpite.

Rodar da raiz do repositório:
    python specs/estudio/gerar_referencias.py
"""
import json
import os
import re
import sys
from collections import defaultdict

RAIZ = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
EXPTYPES = r"F:/PW/1.5.5/EvolvedPWClient/ElementClient/CCommon/ExpTypes.h"
sys.path.insert(0, os.path.join(RAIZ, "specs", "elements_layouts"))
import pw_elements_reader as leitor  # noqa: E402

ESCALARES = {
    "unsigned int": "int", "int": "int", "float": "float", "namechar": "texto", "char": "texto",
    "unsigned short": "short", "short": "short", "unsigned char": "byte", "bool": "byte", "__int64": "int64",
}
NAO_SAO_REGISTROS = re.compile(r"技能|任务|地图|世界|势力|声望|图标|颜色|阵营|帮派|称号|动作|特效|音效|路径|坐标|时间|等级|级别|概率|数量|数目|价格|经验|属性点|策略|部位|职业|区域|组ID|变身|套号|方式的组合|Mask|限制|是否|性别|控制器|驿站|模型|纹理")
# Destino declarado quando o comentário não o diz, com a evidência:
# a forja lista receitas por página (ESTUDIO_DO_PAINEL.md §4.1; `NPC_MAKE_SERVICE` "可生产的商品列表").
DESTINO_DECLARADO = [(re.compile(r"^NPC_MAKE_SERVICE$"), re.compile(r"^pages_\d+_id_goods_\d+$"), "RECIPE")]
# Só "id" como palavra conta como evidência no comentário: "物品" sozinho aparece em quantidades
# ("损毁后掉落的物品数量") e máscaras (proc_type).
CITA = re.compile(r"(?i)(?<![a-z])id(?![a-z])")
BLOCOS_IGNORADOS = []
NOME_DE_ID = re.compile(r"(^|_)id(_|$)")


def ler_defines(texto):
    d = {}
    for m in re.finditer(r"#define\s+(\w+)\s+(\d+)", texto):
        d[m.group(1)] = int(m.group(2))
    return d


def corpo_da_struct(texto, nome):
    m = re.search(r"\bstruct\s+" + re.escape(nome) + r"\s*\{", texto)
    if not m:
        return None
    i, prof = m.end(), 1
    while prof:
        c = texto[i]
        prof += c == "{"
        prof -= c == "}"
        i += 1
    return texto[m.end(): i - 1]


def tamanho(expr, defines):
    expr = expr.strip()
    return int(expr) if expr.isdigit() else defines.get(expr)


def membros(corpo, defines, prefixo=""):
    """Lista achatada [(nome, tipo, comentário)] do corpo de uma struct."""
    saida, i = [], 0
    linhas = corpo.split("\n")
    while i < len(linhas):
        linha = linhas[i]
        if re.match(r"\s*struct\s*\{?\s*(//.*)?$", linha):
            # struct anônima: junta até o `}` que fecha e lê `} nome[N];`
            prof, j, sub = 0, i, []
            while True:
                prof += linhas[j].count("{") - linhas[j].count("}")
                sub.append(linhas[j])
                if prof == 0 and "}" in linhas[j]:
                    break
                j += 1
            fim = re.search(r"\}\s*(\w+)\s*(?:\[\s*(\w+)\s*\])?\s*;", sub[-1])
            if fim is None:
                # bloco fora do padrão `} nome[N];` (struct com nome de tipo, union...): fica de fora
                BLOCOS_IGNORADOS.append(sub[-1].strip())
                i = j + 1
                continue
            interior = "\n".join(sub)
            interior = interior[interior.index("{") + 1: interior.rindex("}")]
            nome, n = fim.group(1), tamanho(fim.group(2), defines) if fim.group(2) else None
            filhos = membros(interior, defines)
            if n is None:
                saida += [(f"{prefixo}{nome}_{f}", t, c) for f, t, c in filhos]
            else:
                for k in range(1, n + 1):
                    saida += [(f"{prefixo}{nome}_{k}_{f}", t, c) for f, t, c in filhos]
            i = j + 1
            continue
        m = re.match(r"\s*((?:unsigned\s+)?\w+)\s+(\w+)\s*((?:\[\s*\w+\s*\])*)\s*;\s*(?://(.*))?", linha)
        if m and m.group(1) in ESCALARES:
            tipo, nome, dims, comentario = ESCALARES[m.group(1)], m.group(2), m.group(3), (m.group(4) or "").strip()
            ns = [tamanho(x, defines) for x in re.findall(r"\[\s*(\w+)\s*\]", dims)]
            if tipo == "texto" or not ns:
                saida.append((prefixo + nome, tipo, comentario))
            elif len(ns) == 1 and ns[0]:
                saida += [(f"{prefixo}{nome}_{k}", tipo, comentario) for k in range(1, ns[0] + 1)]
        i += 1
    return saida


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    texto = open(EXPTYPES, encoding="gbk", errors="replace").read()
    texto = re.sub(r"/\*.*?\*/", "", texto, flags=re.S)
    defines = ler_defines(texto)
    espacos = json.load(open(os.path.join(RAIZ, "specs/elements_layouts/espacos_de_id.json"), encoding="utf-8"))["tabelas"]

    # ids de cada espaço e valores de cada campo, nos dois realms
    ids = defaultdict(set)
    valores = defaultdict(list)
    layouts = {}
    for realm, versao in (("realm_126", 7), ("realm_155", 156)):
        tabelas = leitor.load_elements_data(os.path.join(RAIZ, f"data/{realm}/config/elements.data"))
        layouts[versao] = leitor.load_layout(versao, os.path.join(RAIZ, "specs/elements_layouts"))
        for t, regs in tabelas.items():
            e = espacos.get(t)
            for r in regs:
                if e and "ID" in r:
                    ids[e].add(r["ID"])
                if t == "TALK_PROC" and "id_talk" in r:
                    ids["TALK"].add(r["id_talk"])
                for campo, v in r.items():
                    if campo != "ID" and isinstance(v, int) and v not in (0, -1):
                        valores[(t, campo)].append(v)

    referencias, duvidosos = defaultdict(dict), []
    sem_fonte = 0
    vistos = set()
    for versao, lay in layouts.items():
        for t in lay["tables"]:
            if t.get("variable_size") or (t["name"], "") in vistos:
                continue
            corpo = corpo_da_struct(texto, t["name"])
            fonte = {n: (tp, c) for n, tp, c in membros(corpo, defines)} if corpo else {}
            for f in t["fields"]:
                chave = (t["name"], f["name"])
                if f["type"] != "int32" or f["name"] == "ID" or chave in vistos:
                    continue
                vistos.add(chave)
                tp, comentario = fonte.get(f["name"], (None, None))
                if comentario is None:
                    sem_fonte += 1
                vals = valores.get(chave, [])
                por_nome = bool(NOME_DE_ID.search(f["name"]))
                por_comentario = bool(comentario and CITA.search(comentario))
                if not (por_nome or por_comentario):
                    continue
                if comentario and NAO_SAO_REGISTROS.search(comentario):
                    continue
                if re.search(r"(?i)controller|skill|task", f["name"]):
                    continue  # controladores de mapa, habilidades e missões não são registros do elements
                texto_alvo = f"{f['name']} {comentario or ''}"
                if re.search(r"(?i)附加属性|addon", texto_alvo):
                    alvo = "ADDON"
                elif re.search(r"(?i)配方|recipe", texto_alvo):
                    alvo = "RECIPE"
                elif re.search(r"(?i)对话|dialog", texto_alvo):  # "talk" no nome é do serviço (id_talk_service)
                    alvo = "TALK"
                else:
                    alvo = next((d for rt, rc, d in DESTINO_DECLARADO if rt.match(t["name"]) and rc.match(f["name"])), None)
                if not vals:
                    duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario, "motivo": "sem valor nos dois realms"})
                    continue
                cob = {e: sum(v in s for v in vals) / len(vals) for e, s in ids.items()}
                if alvo is None:
                    # empate de cobertura (os ids de serviço também existem no ADDON): vale a ordem
                    preferencia = ("ESSENCE", "ADDON", "RECIPE", "TALK", "FACE", "CONFIG")
                    alvo = max(cob, key=lambda e: (round(cob[e], 3), -preferencia.index(e) if e in preferencia else -99))
                    # pela cobertura só ESSENCE/ADDON (ids altos); RECIPE/TALK/FACE/CONFIG exigem palavra
                    if alvo not in ("ESSENCE", "ADDON"):
                        duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario,
                                          "motivo": f"só a cobertura apontaria {alvo} (números pequenos coincidem)", "valores": len(vals)})
                        continue
                    # os dois espaços cobrem: sem comentário não há como escolher
                    if comentario is None and min(cob.get("ESSENCE", 0), cob.get("ADDON", 0)) >= 0.95:
                        duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario,
                                          "motivo": "ESSENCE e ADDON cobrem; sem comentário no fonte", "valores": len(vals)})
                        continue
                if t["name"] not in espacos and not re.search(r"(?i)item|material|mine|monster|award", f["name"]):
                    duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario,
                                      "motivo": "tabela sem espaço conhecido: o campo cita as tabelas dela, sem espaço", "valores": len(vals)})
                    continue
                if not por_comentario and sorted(vals)[len(vals) // 2] < 100:
                    duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario,
                                      "motivo": "só o nome diz id e os valores são pequenos", "valores": len(vals)})
                    continue
                if cob.get(alvo, 0) < 0.95:
                    duvidosos.append({"tabela": t["name"], "campo": f["name"], "comentario": comentario,
                                      "motivo": f"cobertura {cob.get(alvo, 0):.2f} em {alvo}", "valores": len(vals)})
                    continue
                referencias[t["name"]][f["name"]] = {
                    "espaco": alvo, "comentario": comentario, "cobertura": round(cob[alvo], 4), "valores": len(vals)}

    saida = {
        "_origem": "gerado por specs/estudio/gerar_referencias.py (B238): membro do ExpTypes.h do cliente 1.5.5 com nome/"
                   "comentário de id E >= 95% dos valores não nulos dos elements.data de realm_126 e realm_155 existindo "
                   "como id no espaço de destino (espacos_de_id.json).",
        "tabelas": {t: dict(sorted(c.items())) for t, c in sorted(referencias.items())},
        "duvidosos": duvidosos,
    }
    destino = os.path.join(RAIZ, "specs/estudio/referencias.json")
    json.dump(saida, open(destino, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    total = sum(len(c) for c in referencias.values())
    por_espaco = defaultdict(int)
    for c in referencias.values():
        for x in c.values():
            por_espaco[x["espaco"]] += 1
    print(f"referências: {total} campos em {len(referencias)} tabelas {dict(por_espaco)}; duvidosos: {len(duvidosos)}; "
          f"campos int32 sem membro no ExpTypes.h: {sem_fonte}; blocos ignorados: {len(BLOCOS_IGNORADOS)} {BLOCOS_IGNORADOS[:3]}")


if __name__ == "__main__":
    main()
