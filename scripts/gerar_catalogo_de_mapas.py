"""Gera web-admin/backend/painel/catalogo_mapas.json: todos os mapas de cada versão (B183).

Fontes:
- Lista e números: o `gs.conf` do pacote original de cada versão. `world_servers` e
  `instance_servers` nomeiam as instâncias; cada `[World_X]`/`[Instance_X]` traz `tag`
  (o número do mapa) e `base_path` (a pasta de dados).
    1.5.5: F:\\PW\\1.5.5\\pwserver_155v156\\home\\pwserver\\gamed\\gs.conf
    1.2.6: <repo>\\..\\files1.2.6\\pwserver\\gamed\\gs.conf
- Nomes: o `instance.txt` do pwadmin do pacote 1.5.5 (F:\\PW\\1.5.5\\home155\\pwadmin),
  `chave=nome`, já em português. Não é dado do cliente; o 1.2.6 usa a mesma lista pela
  mesma chave (decisão do Murillo, B183) — sem nome, fica a chave.

Uso: python scripts/gerar_catalogo_de_mapas.py
"""
import json
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent
GS_CONF = {
    "1.5.5": pathlib.Path(r"F:\PW\1.5.5\pwserver_155v156\home\pwserver\gamed\gs.conf"),
    "1.2.6": REPO.parent / "files1.2.6" / "pwserver" / "gamed" / "gs.conf",
}
NOMES = pathlib.Path(r"F:\PW\1.5.5\home155\pwadmin\instance.txt")
SAIDA = REPO / "web-admin" / "backend" / "painel" / "catalogo_mapas.json"


def ler_gs_conf(caminho):
    secoes, atual, listas = {}, None, {}
    for linha in caminho.read_text(encoding="latin-1").splitlines():
        linha = linha.split("#", 1)[0].strip()
        m = re.match(r"^\[(\w+)\]$", linha)
        if m:
            atual = m.group(1)
            secoes.setdefault(atual, {})
            continue
        m = re.match(r"^(\w+)\s*=\s*(.*)$", linha)
        if not m:
            continue
        chave, valor = m.group(1), m.group(2).strip()
        if chave in ("world_servers", "instance_servers"):
            listas[chave] = [x for x in valor.split(";") if x]
        elif atual is not None:
            secoes[atual][chave] = valor
    mapas = []
    for tipo in ("world_servers", "instance_servers"):
        for inst in listas.get(tipo, []):
            sec = secoes.get(f"World_{inst}") or secoes.get(f"Instance_{inst}") or {}
            if "tag" not in sec:
                continue
            mapas.append({"mapa": int(sec["tag"]), "chave": inst,
                          "pasta": sec.get("base_path", "").strip("/"),
                          "instancia": tipo == "instance_servers"})
    return mapas


def main():
    nomes = {}
    for linha in NOMES.read_text(encoding="latin-1").splitlines():
        if "=" in linha:
            chave, nome = linha.split("=", 1)
            nomes[chave.strip()] = nome.strip()
    catalogo = {}
    for versao, caminho in GS_CONF.items():
        vistos, lista = set(), []
        for m in ler_gs_conf(caminho):
            if m["mapa"] in vistos:
                continue
            vistos.add(m["mapa"])
            m["nome"] = nomes.get(m["chave"], m["chave"])
            lista.append(m)
        catalogo[versao] = sorted(lista, key=lambda m: m["mapa"])
    SAIDA.write_text(json.dumps(catalogo, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print({v: len(l) for v, l in catalogo.items()})


if __name__ == "__main__":
    main()
