"""B323: quantas palavras de 32 bits cada habilidade passa ao `_snwprintf` da descrição — medido no binário.

    python argumentos_da_descricao.py <disasm.txt> <stubs.txt> <iat_snwprintf_hex> <saida.json>

- `disasm.txt`: `dumpbin -disasm` do `elementskill.dll` original;
- `stubs.txt`: `vtable_dos_stubs.exe`;
- `iat_snwprintf_hex`: o slot do `_snwprintf` na tabela de importação (`dumpbin -imports`; no 1.5.5 BR, `1031B02C`).

O `GetIntroduction` de cada stub chama `_snwprintf(buffer, length, format, args…)` (cdecl): o `add esp, N` logo depois
do `call` dá o tamanho dos argumentos — palavras = N/4 − 3 (`double` conta 2). O fonte erra em 221 das 3.316 no 1.5.5
BR (o BR pôs mais números nas descrições), por isso a contagem vem daqui (spec 08 §8).
"""
import json
import re
import sys


def main(disasm, stubs, iat, destino):
    ins, pos = [], {}
    for l in open(disasm, encoding="latin-1"):
        m = re.match(r"\s+([0-9A-F]{8}): (?:[0-9A-F]{2} )+\s+(.*)$", l)
        if m:
            pos[int(m.group(1), 16)] = len(ins)
            ins.append(m.group(2).strip())
    iat = iat.upper()

    def palavras(rva):
        i = pos.get(0x10000000 + rva)
        if i is None:
            return None
        for k in range(i, min(i + 600, len(ins))):
            t = ins[k]
            if t.startswith("call") and iat in t:
                for j in range(k + 1, k + 4):
                    m = re.match(r"add\s+esp,([0-9A-F]+)h?$", ins[j])
                    if m:
                        return int(m.group(1), 16) // 4 - 3
                raise SystemExit(f"call sem add esp em {k}")
            if t.startswith("ret"):
                return None
        return None

    res = {}
    for l in open(stubs):
        p = l.split()
        for rva in p[1:]:
            n = palavras(int(rva, 16))
            if n is not None:
                res[p[0]] = n
                break
    json.dump(res, open(destino, "w"), separators=(",", ":"))
    print(f"{len(res)} habilidades -> {destino}")


if __name__ == "__main__":
    main(*sys.argv[1:])
