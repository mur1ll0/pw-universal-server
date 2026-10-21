"""B314: os formatos de descrição (texto N×10+1 do `skillstr.txt`) para o `gabarito.exe`.

    python formatos.py <skillstr.txt> <formatos.txt>

Linha de saída: "<id> <hex UTF-16LE do formato>". A leitura segue o `AWScriptFile::GetNextToken` (spec 07, B301):
UTF-16LE com BOM, texto entre aspas atravessa linhas e não tem escape.
"""
import re
import sys


def main(origem, destino):
    t = open(origem, "rb").read().decode("utf-16")
    n = 0
    with open(destino, "w", encoding="ascii") as f:
        for m in re.finditer(r'(\d+)\s+"(.*?)"', t, re.S):
            k = int(m.group(1))
            if k % 10 == 1:
                f.write(f"{k // 10} {m.group(2).encode('utf-16-le').hex()}\n")
                n += 1
    print(f"{n} formatos -> {destino}")


if __name__ == "__main__":
    main(*sys.argv[1:])
