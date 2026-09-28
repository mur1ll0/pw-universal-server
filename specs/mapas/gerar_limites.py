"""Gera specs/mapas/limites_<versão>.json: o `limit` de cada mapa do gs.conf original.

`world_manager::InitWorldLimit` (cgame/gs/worldmanager.cpp:108-160) lê a chave `limit` da
seção do mundo ([World_X] ou [Instance_X]) — uma lista separada por `;` (`nofly`, `nothrow`,
`nomount`, `clear-ap`, `allow-root`, …) — e `height_limit`. O catálogo guarda isso **pela tag**
(o worldtag do cliente), como o de terreno. Duas seções com a mesma tag dão uma entrada só.

Uso:
  python specs/mapas/gerar_limites.py 126 ../files1.2.6/pwserver/gamed/gs.conf
  python specs/mapas/gerar_limites.py 155 F:/PW/1.5.5/pwserver_155v156/home/pwserver/gamed/gs.conf
"""
import io
import json
import os
import re
import sys

versao, conf = sys.argv[1], sys.argv[2]
texto = io.open(conf, encoding="gbk", errors="replace").read()

secoes = {}
atual = None
for linha in texto.splitlines():
    linha = linha.split("#", 1)[0].strip()
    m = re.match(r"^\[(\w+)\]$", linha)
    if m:
        atual = m.group(1)
        secoes.setdefault(atual, {})
        continue
    if atual and "=" in linha:
        k, v = linha.split("=", 1)
        secoes[atual][k.strip()] = v.strip()

mapas = {}
for nome, s in secoes.items():
    m = re.match(r"^(World|Instance)_(\w+)$", nome)
    if not m or "tag" not in s:
        continue
    tag = int(s["tag"])
    if tag in mapas:
        continue
    limites = [t.strip() for t in re.split(r"[;,\r\n]", s.get("limit", "")) if t.strip()]
    altura = s.get("height_limit")
    mapas[tag] = {
        "world_id": tag,
        "secao": nome,
        "limites": limites,
        "altura_maxima": float(altura.strip().rstrip(";").rstrip("f")) if altura else 0.0,
    }

saida = {
    "fonte": f"gamed/gs.conf do servidor {versao} (chave `limit` e `height_limit` de [World_*]/[Instance_*]); gerado por specs/mapas/gerar_limites.py",
    "como_ler": "world_id é a `tag` do gs.conf. `limites` são os tokens do `limit` (worldmanager.cpp:108-160): `nofly` proíbe voar (item_flysword.cpp:66, ERR_CANNOT_FLY) e mascote de ar (petman.cpp:104/140/270).",
    "mapas": [mapas[k] for k in sorted(mapas)],
}
destino = os.path.join(os.path.dirname(os.path.abspath(__file__)), f"limites_{versao}.json")
io.open(destino, "w", encoding="utf-8").write(json.dumps(saida, ensure_ascii=False, indent=2) + "\n")
sem_voo = sum(1 for m in mapas.values() if "nofly" in m["limites"])
print(len(mapas), "mapas,", sem_voo, "sem voo ->", destino)
