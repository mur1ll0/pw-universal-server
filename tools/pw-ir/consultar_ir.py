"""Consulta rápida ao IR do protocolo do mundo 3D, sem abrir o JSON de vários MB.

Uso:
    python tools/pw-ir/consultar_ir.py s2c TASK_DATA          # por nome (parcial) ou id
    python tools/pw-ir/consultar_ir.py c2s 41
    python tools/pw-ir/consultar_ir.py struct info_player_1   # struct por nome (parcial)
    python tools/pw-ir/consultar_ir.py s2c 105 --ir specs/protocol/gamedata_153.json

Mostra id, nome, struct do cliente e do servidor, e cada campo com deslocamento,
tamanho e tipo C++. Structs com `bytes: null` têm campos condicionais no fim: o
tamanho-base é a soma dos campos fixos (spec 04 §2).
"""
import argparse
import json
import sys

PADRAO = "specs/protocol/gamedata_155.json"


def mostrar_struct(structs, chave):
    s = structs.get(chave)
    if not s:
        print(f"    (struct {chave} não está no IR)")
        return
    # `bytes` de cada campo já inclui o array.
    base = sum(f["bytes"] for f in s["fields"] if f.get("bytes"))
    print(f"    struct {chave}: bytes={s['bytes']} (base {base}) variavel={s.get('variable')}")
    for f in s["fields"]:
        arr = f"[{f['array_len']}]" if f.get("array_len") else ""
        print(f"      @{f['offset']!s:>4}  {f['bytes']!s:>3}B  {f['cxx']} {f['name']}{arr}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("tipo", choices=["s2c", "c2s", "struct"])
    ap.add_argument("alvo")
    ap.add_argument("--ir", default=PADRAO)
    a = ap.parse_args()
    ir = json.load(open(a.ir, encoding="utf-8"))
    structs = ir["structs"]

    if a.tipo == "struct":
        achou = [k for k in structs if a.alvo.lower() in k.lower()]
        for k in achou:
            mostrar_struct(structs, k)
        if not achou:
            sys.exit(f"nenhuma struct contém {a.alvo!r}")
        return

    comandos = ir["commands"][a.tipo]
    if a.alvo.lstrip("-").isdigit():
        achou = [c for c in comandos if c["id"] == int(a.alvo)]
    else:
        achou = [c for c in comandos if a.alvo.upper() in c["name"].upper()]
    if not achou:
        sys.exit(f"nenhum comando {a.tipo} com {a.alvo!r}")
    for c in achou:
        print(f"{a.tipo.upper()} {c['id']} {c['name']}  payload={c['payload']}  "
              f"servidor={c.get('server_name')}")
        for chave in (c.get("struct"), c.get("server_struct")):
            if chave:
                mostrar_struct(structs, chave)


if __name__ == "__main__":
    main()
