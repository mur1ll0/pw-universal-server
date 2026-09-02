"""
Gera `specs/elements_layouts/v156.json` a partir de `specs/elements_155/PW_1.5.5_v156.cfg`.

Este JSON e o artefato REUTILIZAVEL (o "extrator" propriamente dito de layout): so nome,
campos, tipos e tamanho de cada uma das 231 tabelas do `elements.data` de build v156. Nao
carrega nenhuma correcao de `skip`/`count` especifica de um arquivo -- essas ficam em
`specs/elements_155/realm_155_overrides.json`, separadas de propósito (ver o README da
pasta `elements_layouts/` para o porque).

Rodar de novo se o `.cfg` mudar:
    python specs/elements_layouts/generate_v156.py
"""
import json
import os
import sys

SPECS_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
REPO_ROOT = os.path.abspath(os.path.join(SPECS_DIR, ".."))
sys.path.insert(0, REPO_ROOT)
from specs.elements_155.walk_tables import parse_cfg_full, table_size, CFG_PATH  # noqa: E402


def field_type_and_size(token):
    token = token.strip()
    if token == "int32":
        return "int32", 4
    if token == "float":
        return "float", 4
    if token.startswith("wstring:"):
        return "wstring", int(token.split(":")[1])
    if token.startswith("string:"):
        return "string", int(token.split(":")[1])
    if token.startswith("byte:"):
        return "variable", None
    raise ValueError(f"tipo de campo desconhecido no .cfg: {token}")


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
        "version": 156,
        "header": {
            "size": 8,
            "family": "hex_build",
            "fields": [
                {"name": "version", "type": "uint32", "notes": "byte baixo = numero de build (ex.: 0x3000009c -> 156)"},
                {"name": "build_timestamp", "type": "uint32", "notes": "time_t de quando o arquivo foi gerado, so informativo"},
            ],
        },
        "source_cfg": "specs/elements_155/PW_1.5.5_v156.cfg",
        "table_count": len(tables_out),
        "tables": tables_out,
    }


if __name__ == "__main__":
    layout = build_layout()
    out_path = os.path.join(SPECS_DIR, "elements_layouts", "v156.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(layout, f, ensure_ascii=False, indent=2)
    print(f"escrito {out_path} ({layout['table_count']} tabelas)")
