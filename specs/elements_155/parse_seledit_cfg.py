import re, sys

def field_size(token):
    token = token.strip()
    if token == "int32":
        return 4
    if token == "float":
        return 4
    if token.startswith("wstring:"):
        # Hipotese a testar: N e BYTES (nao caracteres) -- bate com o sizeof real
        # de EQUIPMENT_ADDON (84) compilado de exptypes.h, que tem namechar name[32]
        # (32 caracteres = 64 bytes), e o cfg diz "wstring:64" pra esse mesmo campo.
        return int(token.split(":")[1])
    if token.startswith("string:"):
        return int(token.split(":")[1])
    if token.startswith("byte:"):
        return None  # RAW / tamanho variavel (talk_proc etc.)
    raise ValueError(f"tipo desconhecido: {token}")

def parse_cfg(path):
    lines = open(path, encoding="latin-1").read().splitlines()
    tables = []
    i = 0
    # linha 0 = total, linha 1 = split (onde entra o bloco RAW), depois vem repetindo:
    # nome da tabela / flag / cabecalho de campos / tipos / linha em branco
    total = int(lines[0].strip())
    split_at = int(lines[1].strip())
    i = 2
    while i < len(lines):
        line = lines[i].strip()
        if not line:
            i += 1
            continue
        name = line
        i += 1
        if i >= len(lines):
            break
        flag = lines[i].strip()
        i += 1
        header = lines[i] if i < len(lines) else ""
        i += 1
        types_line = lines[i] if i < len(lines) else ""
        i += 1
        tokens = [t for t in types_line.strip().split(";") if t]
        if tokens == ["byte:AUTO"] or (len(tokens) == 1 and tokens[0].startswith("byte:")):
            size = None
        else:
            try:
                size = sum(field_size(t) for t in tokens)
            except ValueError:
                size = None
        tables.append((name, size, len(tokens)))
    return total, split_at, tables

if __name__ == "__main__":
    path = sys.argv[1]
    total, split_at, tables = parse_cfg(path)
    print(f"# total={total} split_at={split_at} tabelas_lidas={len(tables)}")
    for idx, (name, size, nfields) in enumerate(tables):
        print(f"{idx}\t{name}\t{size}\t{nfields} campos")
