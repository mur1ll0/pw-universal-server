"""
Gera `specs/elements_layouts/v159.json` a partir de `specs/elements_155/PW_1.5.5_v159.cfg`.

Mesma coisa que `generate_v156.py`, só que pra build 159 -- a versão que o client 1.5.5.EN
(`F:\\PW\\1.5.5\\1.5.5.EN\\...`) de fato usa. Ver `docs/ESTADO_E_RETOMADA.md` (seção
"decodificar a build v159") para o porquê: o client rejeita qualquer servidor cujo
`elements.data`/`tasks.data` não bata exatamente com os dele, e o pacote de servidor que
tínhamos (`pwserver_155v156`) é uma build vizinha (156), não a mesma.

Rodar de novo se o `.cfg` mudar:
    python specs/elements_layouts/generate_v159.py
"""
import json
import os
import sys

SPECS_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
REPO_ROOT = os.path.abspath(os.path.join(SPECS_DIR, ".."))
sys.path.insert(0, REPO_ROOT)
from specs.elements_155.walk_tables import parse_cfg_full, table_size  # noqa: E402
from specs.elements_layouts.generate_v156 import field_type_and_size  # noqa: E402

CFG_PATH = os.path.join(SPECS_DIR, "elements_155", "PW_1.5.5_v159.cfg")


def build_layout():
    tables_full = parse_cfg_full(CFG_PATH)
    tables_out = []
    for idx, (name, _flag, fields, tokens) in enumerate(tables_full):
        clean_name = name.split(" - ", 1)[1] if " - " in name else name
        size = table_size(tokens)
        if size is None:
            tables_out.append({
                "index": idx,
                "name": clean_name,
                "variable_size": True,
                "notes": (
                    "Tamanho variavel -- precisa de leitor manual (ver "
                    "read_talk_proc_table em specs/elements_155/walk_tables.py para o "
                    "unico caso conhecido, TALK_PROC)."
                ),
            })
            continue
        field_defs = []
        for fname, tok in zip(fields, tokens):
            ftype, fsize = field_type_and_size(tok)
            field_defs.append({"name": fname, "type": ftype, "size": fsize})
        tables_out.append({
            "index": idx,
            "name": clean_name,
            "variable_size": False,
            "record_size": size,
            "fields": field_defs,
        })
    return {
        "format": "pw_elements_data",
        "version": 159,
        "header": {
            "size": 8,
            "family": "hex_build",
            "fields": [
                {"name": "version", "type": "uint32", "notes": "byte baixo = numero de build (ex.: 0x3000009f -> 159)"},
                {"name": "build_timestamp", "type": "uint32", "notes": "time_t de quando o arquivo foi gerado, so informativo"},
            ],
        },
        "source_cfg": "specs/elements_155/PW_1.5.5_v159.cfg",
        "table_count": len(tables_out),
        "tables": tables_out,
    }


if __name__ == "__main__":
    layout = build_layout()
    out_path = os.path.join(SPECS_DIR, "elements_layouts", "v159.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(layout, f, ensure_ascii=False, indent=2)
    print(f"escrito {out_path} ({layout['table_count']} tabelas)")
