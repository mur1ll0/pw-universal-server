"""Gera os textos de item do painel (B189) a partir do cliente 1.5.5 BR.

- `web-admin/backend/painel/item_desc_indices.json`: nome `ITEMDESC_*` → posição da frase no
  `item_desc.txt`, pela enumeração do cliente (`ElementClient/EC_FixedMsg.h`, a partir de
  `ITEMDESC_ATKSPD_VERYSLOW = 0`, sequencial). O `item_desc.txt` lista as frases nessa ordem
  depois de `#_begin` (`CECGame::Init`, `EC_Game.cpp:521`).
- Os textos (`item_desc.txt`, `item_ext_desc.txt`, `item_color.txt`, `item_ext_prop.txt`) são
  extraídos do `configs.pck` para `data/textos/` (fora do git) com `tools/pw-pck-extract`.

Uso: python scripts/gerar_textos_de_itens.py
"""
import json
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
CABECALHO = pathlib.Path(r"F:\PW\1.5.5\EvolvedPWClient\ElementClient\EC_FixedMsg.h")
CONFIGS = pathlib.Path(r"F:\PW\1.5.5\1.5.5 BR\Perfect World 1.5.5 BR\element\configs.pck")
TEXTOS = ["item_desc.txt", "item_ext_desc.txt", "item_color.txt", "item_ext_prop.txt"]


def indices_itemdesc():
    texto = CABECALHO.read_text(encoding="latin-1")
    inicio = texto.index("ITEMDESC_ATKSPD_VERYSLOW")
    corpo = texto[inicio:texto.index("};", inicio)]
    indices, n = {}, 0
    for linha in corpo.splitlines():
        linha = linha.split("//")[0].strip().rstrip(",")
        m = re.match(r"^(ITEMDESC_\w+)\s*(?:=\s*(\d+))?$", linha)
        if not m:
            continue
        if m.group(2) is not None:
            n = int(m.group(2))
        indices[m.group(1)] = n
        n += 1
    return indices


def main():
    destino = REPO / "data" / "textos"
    destino.mkdir(parents=True, exist_ok=True)
    for nome in TEXTOS:
        subprocess.run([sys.executable, str(REPO / "tools" / "pw-pck-extract" / "pw_pck_extract.py"), "get",
                        str(CONFIGS), "configs\\" + nome, str(destino / nome)], check=True, capture_output=True)
    indices = indices_itemdesc()
    saida = REPO / "web-admin" / "backend" / "painel" / "item_desc_indices.json"
    saida.write_text(json.dumps(indices, indent=1) + "\n", encoding="utf-8")
    print(len(indices), "ITEMDESC_*; maior índice", max(indices.values()))


if __name__ == "__main__":
    main()
