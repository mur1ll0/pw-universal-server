"""
Cruza um export de tabela do editor `EDITOR DE ELEMENTS 1.5.5 ADMVAL`
(`D:\\PROJETOS\\PWPRIVATE\\Tools\\EDITOR DE ELEMENTS 1.5.5 ADMVAL`) contra o
`elements.data` real, achando a posicao exata da tabela por "impressao digital" binaria --
muito mais confiavel que tentar adivinhar por pontuacao de plausibilidade (ver README.md,
secao "Avanco decisivo").

O QUE E O EXPORT: o ADMVAL exporta uma tabela como texto UTF-16LE, uma linha por campo, no
formato `indice_tabela@linha@indice_campo@valor` (ex.: `154@72@0@0@10` = tabela 72, linha 0,
campo 0, valor `10`). `indice_tabela` bate com o indice 0-based do `.cfg`
(`PW_1.5.5_v156.cfg`), e o primeiro numero de cada bloco e o total de campos da tabela --
confirma o `.cfg` de graca.

A TECNICA: construir a sequencia de bytes dos campos NUMERICOS (`int32`/`float`) de uma
linha do export, ignorando `Name`/campos texto (podem estar traduzidos entre o arquivo do
client usado no export e o `elements.data` que se quer resolver), e procurar essa sequencia
EXATA no arquivo alvo. Com uma janela de ~30+ campos contiguos a ocorrencia costuma ser
UNICA no arquivo inteiro -- um fingerprint monta muito mais forte que "parece plausivel".

Uso:
    python specs/elements_155/crossref_admval.py <indice_da_tabela_no_cfg> <export.txt> [elements.data]

O export deve ja estar decodificado de UTF-16LE pra UTF-8 (o `.data` que o ADMVAL gera vem
com BOM UTF-16LE; decodifique com algo como
`open(f, 'rb').read().decode('utf-16le')` antes de salvar como `.txt`).
"""
import struct
import sys
import os

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, REPO_ROOT)
from specs.elements_155.walk_tables import parse_cfg_full, table_size, decode_record, CFG_PATH, DEFAULT_DATA_PATH


def load_gt(path):
    """Le um export do ADMVAL (`indice_tabela@linha@campo@valor` por linha) e devolve
    {linha: {campo: valor_str}}."""
    rows = {}
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line:
            continue
        parts = line.split("@")
        row, fidx, val = int(parts[2]), int(parts[3]), parts[4]
        rows.setdefault(row, {})[fidx] = val
    return rows


def build_pattern(tokens, row_vals, start_field, end_field):
    pat = b""
    for i in range(start_field, end_field):
        tok = tokens[i]
        v = row_vals[i]
        if tok == "int32":
            pat += struct.pack("<i", int(v))
        elif tok == "float":
            pat += struct.pack("<f", float(v))
        else:
            return None  # campo string/wstring -- nao entra num fingerprint binario cru
    return pat


def find_table(idx, gt_path, data_path=None):
    tables = parse_cfg_full(CFG_PATH)
    name, _flag, fields, tokens = tables[idx]
    size = table_size(tokens)
    buf = open(data_path or DEFAULT_DATA_PATH, "rb").read()
    rows = load_gt(gt_path)
    nrows_gt = len(rows)
    out = [f"=== [{idx}] {name} size={size} linhas_no_export={nrows_gt} ==="]

    row0 = rows[0]
    n = len(tokens)
    runs = []
    start = 0
    while start < n:
        if tokens[start] not in ("int32", "float") or start not in row0:
            start += 1
            continue
        end = start
        while end < n and tokens[end] in ("int32", "float") and end in row0:
            end += 1
        runs.append((start, end))
        start = end + 1
    if not runs:
        out.append("  nenhum campo numerico com valor no export -- tente a tecnica de IDs em sequencia (ver PET_TYPE no README)")
        print("\n".join(out))
        return None

    fs, fe = max(runs, key=lambda r: r[1] - r[0])
    if fe - fs < 4:
        out.append(f"  maior trecho numerico contiguo tem so {fe - fs} campos -- fingerprint fraco, risco de falso positivo")
    pattern = build_pattern(tokens, row0, fs, fe)
    occurrences = []
    i = buf.find(pattern)
    while i != -1:
        occurrences.append(i)
        i = buf.find(pattern, i + 1)
    out.append(f"  fingerprint campos [{fs}:{fe}) ({len(pattern)} bytes) -> {len(occurrences)} ocorrencia(s): {occurrences}")
    if len(occurrences) != 1:
        out.append("  AMBIGUO ou NAO ENCONTRADO -- nao da pra confiar, precisa de outro trecho/tecnica")
        print("\n".join(out))
        return None

    off_in_rec = 0
    for i in range(fs):
        off_in_rec += 4 if tokens[i] in ("int32", "float") else int(tokens[i].split(":")[1])
    record_start = occurrences[0] - off_in_rec
    count_off = record_start - 4
    count_val = struct.unpack_from("<I", buf, count_off)[0]
    out.append(f"  record0_start={record_start}  count_field_offset={count_off}  count_no_arquivo_alvo={count_val}")

    rec = buf[record_start:record_start + size]
    vals, _sc, _ = decode_record(rec, fields, tokens)
    numeric_total = sum(1 for t in tokens if t in ("int32", "float"))
    mismatches = []
    for i, v in enumerate(vals):
        if tokens[i] not in ("int32", "float"):
            continue
        gtval = row0.get(i)
        try:
            ok = abs(float(v) - float(gtval)) < 1e-3
        except Exception:
            ok = False
        if not ok:
            mismatches.append((i, v, gtval))
    out.append(f"  campos numericos batendo: {numeric_total - len(mismatches)}/{numeric_total}  mismatches={mismatches[:10]}")

    if 1 in rows and all(i in rows[1] for i in range(fs, fe)):
        pattern1 = build_pattern(tokens, rows[1], fs, fe)
        row1_expected_pos = record_start + size + off_in_rec
        actual = buf[row1_expected_pos: row1_expected_pos + len(pattern1)]
        out.append(f"  verificacao registro1 na posicao esperada (stride={size}): {'OK' if actual == pattern1 else 'NAO BATEU'}")

    out.append(f"  linhas no export do ADMVAL: {nrows_gt}")
    print("\n".join(out))
    return record_start, count_off, count_val, nrows_gt


if __name__ == "__main__":
    table_idx = int(sys.argv[1])
    export_path = sys.argv[2]
    data_arg = sys.argv[3] if len(sys.argv) > 3 else None
    find_table(table_idx, export_path, data_arg)
