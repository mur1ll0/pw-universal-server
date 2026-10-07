"""Confere no ElementClient 1.5.5 BR as frases que cada efeito de item usa na dica (B193).

O `item_desc.txt` BR só bate com a enumeração `ITEMDESC_*` do fonte (`EC_FixedMsg.h:375`) até o
índice 112 (B189); dali em diante o binário é o juiz. Este script desmonta o `switch` do
`CECIvtrEquip::AddOneAddOnPropDesc` no executável e, para cada tipo de efeito, lê as chaves que o
caso passa à tabela de frases. Grava `web-admin/backend/painel/efeitos_no_binario_br.json`:

- `frases`: `ITEMDESC_*` → posição real no `item_desc.txt` BR, para as frases além do 112;
- `no_default`: tipos que o binário BR manda ao `default:` (`ITEMDESC_ERRORPROP`) — casos que o
  fonte tem e este cliente não;
- `conferidos`: quantos tipos tiveram as chaves do binário iguais às do fonte.

Despacho (`0x51e320-0x51e337`): `byPropType & ebx(0xff)`, `cmp 0xd4`, índice em 0x522ac4, saltos em
0x522884; afiador desviado antes (`call 0x52e760`). A frase sai por três formas:
- `mov [esp+X], chave|ebx` e `call 0x4f2e00` (busca na tabela expandida);
- `mov [esp+X], chave` e `call 0x58a200` (outra busca expandida);
- `push chave` e `jmp` ao bloco comum ou `call 0x522bc0` (GetWideString).
`ebx` = 0xff na entrada (`0x51e2f4`) = chave 255 enquanto o caso não o escreve.

Requer `pefile` e `capstone`. Uso: python scripts/conferir_efeitos_no_binario.py
"""

import json, re, struct, sys
import pefile
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
sys.stdout.reconfigure(encoding="utf-8")
EXE = r"F:\PW\1.5.5\1.5.5 BR\Perfect World 1.5.5 BR\element\ELEMENTCLIENT.EXE"
R = r"F:\Python_C_Projects\PWSource1.5.3\pw-universal-server"
pe = pefile.PE(EXE, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
ler = lambda va, n: img[va - base: va - base + n]
MAXIMO, INDICE, TABELA, FIM = 0xd4, 0x522ac4, 0x522884, 0x522884
BUSCAS = {"0x4f2e00", "0x58a200"}
alvos = {k: struct.unpack("<I", ler(TABELA + 4 * ler(INDICE + k, 1)[0], 4))[0] for k in range(MAXIMO + 1)}
inicios = sorted(set(alvos.values()) | {FIM})
md = Cs(CS_ARCH_X86, CS_MODE_32)
NUM = r"0x[0-9a-f]+|\d+"


def chaves(ini):
    fim = next(a for a in inicios if a > ini)
    ins = list(md.disasm(ler(ini, fim - ini), ini))
    saida, ebx_mexido = [], False
    for n, x in enumerate(ins):
        if re.match(r"(mov|xor|add|sub|pop|lea|and|or|inc|dec) ebx\b", f"{x.mnemonic} {x.op_str}"):
            ebx_mexido = True
        if x.mnemonic == "push" and re.fullmatch(NUM, x.op_str):
            prox = ins[n + 1:n + 4]
            if prox and prox[0].mnemonic == "jmp":
                saida.append(int(x.op_str, 0))
                continue
            for y in prox:
                if y.mnemonic == "push":
                    break
                if y.mnemonic == "call" and y.op_str == "0x522bc0":
                    saida.append(int(x.op_str, 0))
                    break
        if x.mnemonic == "call" and x.op_str in BUSCAS:
            for y in reversed(ins[max(0, n - (12 if x.op_str == "0x58a200" else 4)):n]):
                m = re.fullmatch(r"dword ptr \[esp \+ 0x[0-9a-f]+\], (" + NUM + r"|ebx)", y.op_str)
                if y.mnemonic == "mov" and m:
                    v = m.group(1)
                    if v == "ebx":
                        if ebx_mexido:
                            continue  # `ebx` já não é 0xff: não é a chave
                        saida.append(255)
                    else:
                        saida.append(int(v, 0))
                    break
    return sorted(set(s for s in saida if s < 1000))


def main():
    tabela = json.load(open(R + r"\web-admin\backend\painel\efeitos_do_cliente.json", encoding="utf-8"))
    idx = json.load(open(R + r"\web-admin\backend\painel\item_desc_indices.json", encoding="utf-8"))
    padrao = 0x522849  # `ja 0x522849` (0x51e329)
    assert chaves(padrao) == [idx["ITEMDESC_ERRORPROP"]], "o default devia usar ERRORPROP"
    frases, no_default, conferidos, conflitos = {}, [], 0, []
    for k in sorted(int(c) for c in tabela):
        if k > MAXIMO:
            continue
        fonte = sorted({p[0] for p in tabela[str(k)]["partes"] if p[0]}, key=lambda n: idx[n])
        b = chaves(alvos[k])
        if alvos[k] == padrao:
            no_default.append(k)
            continue
        seguros = {idx[n] for n in fonte if idx[n] <= 112}
        resto = [n for n in fonte if idx[n] > 112]
        livres = [c for c in b if c not in seguros and c > 112]
        if not resto:
            conferidos += set(b) - {0} <= seguros and (seguros <= set(b) or not b)
            continue
        if len(resto) == 1 and len(livres) == 1:
            if frases.get(resto[0], livres[0]) != livres[0]:
                conflitos.append((k, resto[0], frases[resto[0]], livres[0]))
            frases[resto[0]] = livres[0]
        else:
            conflitos.append((k, resto, b))
    saida = {"frases": frases, "no_default": no_default, "conferidos": conferidos,
             "fonte": "ElementClient 1.5.5 BR, AddOneAddOnPropDesc (switch em 0x51e337)"}
    json.dump(saida, open(R + r"\web-admin\backend\painel\efeitos_no_binario_br.json", "w", encoding="utf-8"),
              ensure_ascii=False, indent=1)
    print(f"frases {len(frases)}; no default {no_default}; conferidos {conferidos}; conflitos {conflitos}")


if __name__ == "__main__":
    main()
