"""Gera specs/mapas/terreno_155.json a partir do gs.conf do pacote pwserver_155v156.

Cada mapa do original é uma seção [World_X] ou [Instance_X] (com `tag` e `base_path`) e uma
[Terrain_X] com a grade de `.hmap`. O `world_id` do catálogo é o **tag** — o número que o
cliente recebe em `worldtag` e que o `INST_DATA_CHECKOUT` carrega. Não o `index`, que é a
posição da seção no arquivo: a primeira versão deste catálogo (2026-09-11, feita à mão)
usou o `index`, e só o mundo 1 saiu certo porque ali os dois valem 1.

Duas seções com o mesmo tag (gs01 e gs02__ são o mundo 1 dividido em dois processos)
dão uma entrada só. Várias instâncias que dividem a mesma pasta (is72..is75 em a72/)
ganham uma entrada cada, pelo tag.

Uso: python specs/mapas/gerar_terreno_155.py F:/PW/1.5.5/pwserver_155v156/home/pwserver/gamed/gs.conf
"""
import io
import json
import os
import re
import sys

conf = sys.argv[1] if len(sys.argv) > 1 else "F:/PW/1.5.5/pwserver_155v156/home/pwserver/gamed/gs.conf"
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

def f(v):
    return float(v.rstrip("f"))

mapas = {}
for nome, s in secoes.items():
    m = re.match(r"^(World|Instance)_(\w+)$", nome)
    if not m or "tag" not in s or "base_path" not in s:
        continue
    t = secoes.get("Terrain_" + m.group(2))
    if not t:
        continue
    tag = int(s["tag"])
    if tag in mapas:
        continue
    mapas[tag] = {
        "world_id": tag,
        "base_path": s["base_path"].rstrip("/"),
        "sub_pasta": t["szMapPath"],
        "blocos_colunas": int(t["nNumCols"]),
        "blocos_linhas": int(t["nNumRows"]),
        "vertices_por_bloco": int(t["nAreaWidth"]),
        "tamanho_da_celula": f(t["vGridSize"]),
        "altura_minima": f(t["vHeightMin"]),
        "altura_maxima": f(t["vHeightMax"]),
        "blocos": int(t["nNumAreas"]),
    }

saida = {
    "fonte": "gamed/gs.conf do pacote pwserver_155v156 (seções [World_*]/[Instance_*] e [Terrain_*]); gerado por specs/mapas/gerar_terreno_155.py",
    "como_ler": "world_id é o `tag` do gs.conf (o worldtag do cliente). Cada mapa é uma grade de blocos_colunas x blocos_linhas arquivos <base_path>/<sub_pasta>/<n>.hmap, n de 1 a blocos. Cada arquivo tem (vertices_por_bloco+1)^2 floats little-endian no intervalo 0..1, que viram altura real por h*(altura_maxima-altura_minima)+altura_minima. Ver crates/pw-data-loader/src/terreno.rs e, do lado do original, cgame/gs/terrain.cpp.",
    "mapas": [mapas[k] for k in sorted(mapas)],
}
destino = os.path.join(os.path.dirname(os.path.abspath(__file__)), "terreno_155.json")
io.open(destino, "w", encoding="utf-8").write(json.dumps(saida, ensure_ascii=False, indent=2) + "\n")
print(len(mapas), "mapas ->", destino)
