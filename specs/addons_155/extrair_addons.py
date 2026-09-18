r"""Extrai do servidor 1.5.5 qual tratador cada id de propriedade adicional (addon) usa.

Fonte: `F:\PW\1.5.5\EvolvedPWServer\cgame\gs\item\item_addon.cpp`, a função que
registra os tratadores: `INSERT_ADDON(id, tratador)` e `MY_INSERT_ADDON(tipo)`
(`item_addon.cpp:1516-2950`). O tratador decide como os parâmetros do `EQUIPMENT_ADDON` do
`elements.data` são sorteados (`GenerateParam`) e o que o addon faz: mexe na essência ao
gerar (`ApplyAtGeneration`), no item (`UpdateItem`) ou no jogador ao vestir (`Activate`).

Saída: `addons.json` = `{"fonte": ..., "tratadores": {"<id>": "<tratador sem espaços>"}}`.

Uso:
    python specs/addons_155/extrair_addons.py F:/PW/1.5.5/EvolvedPWServer/cgame/gs/item/item_addon.cpp
"""
import json
import os
import re
import sys


def main():
    fonte = sys.argv[1] if len(sys.argv) > 1 else r"F:\PW\1.5.5\EvolvedPWServer\cgame\gs\item\item_addon.cpp"
    texto = open(fonte, encoding="latin1").read()
    # Só o que não está comentado.
    texto = re.sub(r"/\*.*?\*/", "", texto, flags=re.S)
    texto = re.sub(r"//[^\n]*", "", texto)
    saida = {}
    for m in re.finditer(r"\bINSERT_ADDON\s*\(\s*(\d+)\s*,\s*(.+?)\)\s*;", texto):
        saida[m.group(1)] = re.sub(r"\s+", "", m.group(2))
    destino = os.path.join(os.path.dirname(os.path.abspath(__file__)), "addons.json")
    with open(destino, "w", encoding="utf-8") as f:
        json.dump({"fonte": "EvolvedPWServer/cgame/gs/item/item_addon.cpp", "tratadores": saida}, f, ensure_ascii=False, indent=0, sort_keys=True)
    print(f"{len(saida)} addons -> {destino}")


if __name__ == "__main__":
    main()
